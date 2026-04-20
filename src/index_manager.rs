use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use git2::Delta;
use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::AllQuery,
    Index, IndexWriter, Term,
};
use walkdir::WalkDir;

use crate::frontmatter;
use crate::git;
use crate::index_schema::IndexSchema;
use crate::links;
use crate::slug::Slug;
use crate::type_registry::SpaceTypeRegistry;

// ── Return types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexReport {
    pub wiki: String,
    pub pages_indexed: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateReport {
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    pub wiki: String,
    pub path: String,
    pub built: Option<String>,
    pub pages: usize,
    pub sections: usize,
    pub stale: bool,
    pub openable: bool,
    pub queryable: bool,
}

/// Optional context for auto-recovery on corrupt index.
pub struct RecoveryContext<'a> {
    pub wiki_root: &'a Path,
    pub repo_root: &'a Path,
    pub registry: &'a SpaceTypeRegistry,
}

// ── state.toml ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexState {
    #[serde(default)]
    pub schema_hash: String,
    pub built: String,
    pub pages: usize,
    pub sections: usize,
    pub commit: String,
    #[serde(default)]
    pub types: std::collections::HashMap<String, String>,
}

// ── SpaceIndexManager ─────────────────────────────────────────────────────────

pub struct SpaceIndexManager {
    wiki_name: String,
    index_path: PathBuf,
}

impl SpaceIndexManager {
    pub fn new(wiki_name: impl Into<String>, index_path: impl Into<PathBuf>) -> Self {
        Self {
            wiki_name: wiki_name.into(),
            index_path: index_path.into(),
        }
    }

    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    pub fn wiki_name(&self) -> &str {
        &self.wiki_name
    }

    pub fn last_commit(&self) -> Option<String> {
        let state_path = self.index_path.join("state.toml");
        let content = std::fs::read_to_string(&state_path).ok()?;
        let state: IndexState = toml::from_str(&content).ok()?;
        if state.commit.is_empty() {
            None
        } else {
            Some(state.commit)
        }
    }

    pub fn rebuild(
        &self,
        wiki_root: &Path,
        repo_root: &Path,
        is: &IndexSchema,
        registry: &SpaceTypeRegistry,
    ) -> Result<IndexReport> {
        let start = std::time::Instant::now();

        let search_dir = self.index_path.join("search-index");
        std::fs::create_dir_all(&search_dir)?;

        let dir = MmapDirectory::open(&search_dir)
            .with_context(|| format!("failed to open index dir: {}", search_dir.display()))?;
        let index = Index::open_or_create(dir, is.schema.clone())?;
        let mut writer: IndexWriter = index.writer(50_000_000)?;
        writer.delete_all_documents()?;

        let mut pages = 0usize;
        let mut sections = 0usize;

        for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let slug = match Slug::from_path(path, wiki_root) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let uri = format!("wiki://{}/{slug}", self.wiki_name);
            let page = frontmatter::parse(&content);

            writer.add_document(build_document(is, registry, slug.as_str(), &uri, &page))?;

            if page.page_type() == Some("section") {
                sections += 1;
            }
            pages += 1;
        }

        writer.commit()?;

        let commit = git::current_head(repo_root).unwrap_or_default();
        let state = IndexState {
            schema_hash: registry.schema_hash().to_string(),
            built: Utc::now().to_rfc3339(),
            pages,
            sections,
            commit,
            types: registry.type_hashes().clone(),
        };
        std::fs::write(
            self.index_path.join("state.toml"),
            toml::to_string_pretty(&state)?,
        )?;

        Ok(IndexReport {
            wiki: self.wiki_name.clone(),
            pages_indexed: pages,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub fn update(
        &self,
        wiki_root: &Path,
        repo_root: &Path,
        last_indexed_commit: Option<&str>,
        is: &IndexSchema,
        registry: &SpaceTypeRegistry,
    ) -> Result<UpdateReport> {
        let changes = git::collect_changed_files(repo_root, wiki_root, last_indexed_commit)?;
        if changes.is_empty() {
            return Ok(UpdateReport::default());
        }

        let search_dir = self.index_path.join("search-index");
        let dir = MmapDirectory::open(&search_dir)
            .with_context(|| format!("failed to open index dir: {}", search_dir.display()))?;
        let index = Index::open(dir).context("failed to open index")?;
        let mut writer: IndexWriter = index.writer(50_000_000)?;

        let f_slug = is.field("slug");
        let wiki_prefix = wiki_root
            .strip_prefix(repo_root)
            .unwrap_or(Path::new("wiki"));
        let mut updated = 0;
        let mut deleted = 0;

        for (path, status) in &changes {
            let slug = match Slug::from_path(path, wiki_prefix) {
                Ok(s) => s,
                Err(_) => continue,
            };

            writer.delete_term(Term::from_field_text(f_slug, slug.as_str()));

            if *status == Delta::Deleted {
                deleted += 1;
            } else {
                let full_path = repo_root.join(path);
                if let Ok(content) = std::fs::read_to_string(&full_path) {
                    let page = frontmatter::parse(&content);
                    let uri = format!("wiki://{}/{slug}", self.wiki_name);
                    writer.add_document(build_document(is, registry, slug.as_str(), &uri, &page))?;
                    updated += 1;
                }
            }
        }

        writer.commit()?;
        Ok(UpdateReport { updated, deleted })
    }

    pub fn status(&self, repo_root: &Path) -> Result<IndexStatus> {
        let state_path = self.index_path.join("state.toml");
        let search_dir = self.index_path.join("search-index");

        let (built, pages, sections, stale) = if state_path.exists() {
            match std::fs::read_to_string(&state_path)
                .ok()
                .and_then(|c| toml::from_str::<IndexState>(&c).ok())
            {
                Some(state) => {
                    let head = git::current_head(repo_root).unwrap_or_default();
                    let (current_schema_hash, _) =
                        crate::type_registry::compute_disk_hashes(repo_root).unwrap_or_default();
                    let stale =
                        state.commit != head || state.schema_hash != current_schema_hash;
                    (Some(state.built), state.pages, state.sections, stale)
                }
                None => (None, 0, 0, true),
            }
        } else {
            (None, 0, 0, true)
        };

        let (openable, queryable) = if search_dir.exists() {
            let try_open = || -> std::result::Result<Index, Box<dyn std::error::Error>> {
                let dir = MmapDirectory::open(&search_dir)?;
                Ok(Index::open(dir)?)
            };
            match try_open() {
                Ok(index) => {
                    let queryable = index
                        .reader()
                        .map(|r| {
                            r.searcher()
                                .search(&AllQuery, &TopDocs::with_limit(1))
                                .is_ok()
                        })
                        .unwrap_or(false);
                    (true, queryable)
                }
                Err(_) => (false, false),
            }
        } else {
            (false, false)
        };

        Ok(IndexStatus {
            wiki: self.wiki_name.clone(),
            path: search_dir.to_string_lossy().into(),
            built,
            pages,
            sections,
            stale,
            openable,
            queryable,
        })
    }

    pub fn delete_by_type(&self, is: &IndexSchema, type_name: &str) -> Result<()> {
        let search_dir = self.index_path.join("search-index");
        let dir = MmapDirectory::open(&search_dir)
            .with_context(|| format!("failed to open index dir: {}", search_dir.display()))?;
        let index = Index::open(dir).context("failed to open index")?;
        let mut writer: IndexWriter = index.writer(50_000_000)?;
        let f_type = is.field("type");
        writer.delete_term(Term::from_field_text(f_type, type_name));
        writer.commit()?;
        Ok(())
    }

    pub fn open_index(
        &self,
        is: &IndexSchema,
        recovery: Option<&RecoveryContext<'_>>,
    ) -> Result<Index> {
        let search_dir = self.index_path.join("search-index");
        let try_open = || -> Result<Index> {
            let dir = MmapDirectory::open(&search_dir)?;
            Ok(Index::open(dir)?)
        };

        match try_open() {
            Ok(idx) => Ok(idx),
            Err(e) => {
                if let Some(ctx) = recovery {
                    tracing::warn!(
                        wiki = %self.wiki_name,
                        error = %e,
                        "index corrupt, rebuilding",
                    );
                    if search_dir.exists() {
                        let _ = std::fs::remove_dir_all(&search_dir);
                    }
                    self.rebuild(ctx.wiki_root, ctx.repo_root, is, ctx.registry)?;
                    try_open().context("index still corrupt after rebuild")
                } else {
                    Err(e)
                }
            }
        }
    }
}

// ── Deprecated free-function wrappers ─────────────────────────────────────────

#[deprecated(note = "use SpaceIndexManager::last_commit()")]
pub fn last_indexed_commit(index_path: &Path) -> Option<String> {
    SpaceIndexManager::new("", index_path).last_commit()
}

#[deprecated(note = "use SpaceIndexManager::rebuild()")]
pub fn rebuild_index(
    wiki_root: &Path,
    index_path: &Path,
    wiki_name: &str,
    repo_root: &Path,
    is: &IndexSchema,
    registry: &SpaceTypeRegistry,
) -> Result<IndexReport> {
    SpaceIndexManager::new(wiki_name, index_path).rebuild(wiki_root, repo_root, is, registry)
}

#[deprecated(note = "use SpaceIndexManager::update()")]
pub fn update_index(
    wiki_root: &Path,
    index_path: &Path,
    repo_root: &Path,
    last_indexed_commit: Option<&str>,
    is: &IndexSchema,
    wiki_name: &str,
    registry: &SpaceTypeRegistry,
) -> Result<UpdateReport> {
    SpaceIndexManager::new(wiki_name, index_path).update(
        wiki_root,
        repo_root,
        last_indexed_commit,
        is,
        registry,
    )
}

#[deprecated(note = "use SpaceIndexManager::status()")]
pub fn index_status(wiki_name: &str, index_path: &Path, repo_root: &Path) -> Result<IndexStatus> {
    SpaceIndexManager::new(wiki_name, index_path).status(repo_root)
}

#[deprecated(note = "use SpaceIndexManager::delete_by_type()")]
pub fn delete_by_type(
    index_path: &Path,
    is: &IndexSchema,
    type_name: &str,
) -> Result<()> {
    SpaceIndexManager::new("", index_path).delete_by_type(is, type_name)
}

#[deprecated(note = "use SpaceIndexManager::open_index()")]
pub fn open_index(
    search_dir: &Path,
    index_path: &Path,
    wiki_name: &str,
    is: &IndexSchema,
    recovery: Option<&RecoveryContext<'_>>,
) -> Result<Index> {
    // The old API took search_dir separately; the new API derives it from index_path.
    // For backward compat, if search_dir != index_path/"search-index", we use index_path
    // that would produce the correct search_dir. Since the old callers always pass
    // index_path.join("search-index") as search_dir, delegating directly works.
    let _ = search_dir; // old callers pass index_path.join("search-index")
    SpaceIndexManager::new(wiki_name, index_path).open_index(is, recovery)
}

// ── Document building (private) ───────────────────────────────────────────────

fn build_document(
    is: &IndexSchema,
    registry: &SpaceTypeRegistry,
    slug: &str,
    uri: &str,
    page: &frontmatter::ParsedPage,
) -> tantivy::TantivyDocument {
    let mut doc = tantivy::TantivyDocument::default();

    // Fixed fields
    doc.add_text(is.field("slug"), slug);
    doc.add_text(is.field("uri"), uri);

    // Get aliases for this page's type
    let page_type = page.page_type().unwrap_or("page");
    let empty_aliases = std::collections::HashMap::new();
    let aliases = registry.aliases(page_type).unwrap_or(&empty_aliases);

    // Dynamic frontmatter indexing with alias resolution.
    let mut indexed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut extra_text = String::new();

    for (field_name, value) in &page.frontmatter {
        if aliases.contains_key(field_name.as_str()) {
            continue;
        }

        let canonical = field_name.as_str();
        indexed.insert(canonical.to_string());
        index_value(&mut doc, &mut extra_text, is, canonical, value);
    }

    // Handle aliased source fields whose canonical target was NOT present
    for (source_field, canonical) in aliases {
        if indexed.contains(canonical.as_str()) {
            continue;
        }
        if let Some(value) = page.frontmatter.get(source_field.as_str()) {
            indexed.insert(canonical.clone());
            index_value(&mut doc, &mut extra_text, is, canonical, value);
        }
    }

    // Body + unrecognized fields
    if extra_text.is_empty() {
        doc.add_text(is.field("body"), &page.body);
    } else {
        doc.add_text(is.field("body"), format!("{}\n{}", page.body, extra_text.trim()));
    }

    // Body wiki-links
    for link in links::extract_body_wikilinks(&page.body) {
        doc.add_text(is.field("body_links"), &link);
    }

    doc
}

fn index_value(
    doc: &mut tantivy::TantivyDocument,
    extra_text: &mut String,
    is: &IndexSchema,
    canonical: &str,
    value: &serde_yaml::Value,
) {
    if let Some(field_handle) = is.try_field(canonical) {
        if is.is_keyword(canonical) {
            for s in yaml_to_strings(value) {
                doc.add_text(field_handle, &s);
            }
        } else {
            let text = yaml_to_text(value);
            if !text.is_empty() {
                doc.add_text(field_handle, &text);
            }
        }
    } else {
        let text = yaml_to_text(value);
        if !text.is_empty() {
            extra_text.push(' ');
            extra_text.push_str(&text);
        }
    }
}

fn yaml_to_text(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| match v {
                serde_yaml::Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        serde_yaml::Value::Mapping(_) => {
            serde_json::to_string(value).unwrap_or_default()
        }
        serde_yaml::Value::Null => String::new(),
        _ => String::new(),
    }
}

fn yaml_to_strings(value: &serde_yaml::Value) -> Vec<String> {
    match value {
        serde_yaml::Value::String(s) => vec![s.clone()],
        serde_yaml::Value::Bool(b) => vec![b.to_string()],
        serde_yaml::Value::Number(n) => vec![n.to_string()],
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| match v {
                serde_yaml::Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        serde_yaml::Value::Null => vec![],
        _ => vec![yaml_to_text(value)],
    }
}
