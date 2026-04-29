use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use chrono::Utc;
use git2::Delta;
use serde::{Deserialize, Serialize};
use tantivy::{
    Index, IndexReader, IndexWriter, Searcher, Term, collector::TopDocs, directory::MmapDirectory,
    query::AllQuery,
};
use walkdir::WalkDir;

use crate::frontmatter;
use crate::git;
use crate::index_schema::IndexSchema;
use crate::links;
use crate::slug::Slug;
use crate::type_registry::SpaceTypeRegistry;

// ── Return types ──────────────────────────────────────────────────────────────

/// Result of a full index rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexReport {
    /// Name of the wiki that was indexed.
    pub wiki: String,
    /// Number of pages successfully added to the index.
    pub pages_indexed: usize,
    /// Number of files that were skipped due to read errors or invalid paths.
    pub skipped: usize,
    /// Wall-clock time taken for the rebuild in milliseconds.
    pub duration_ms: u64,
}

/// Result of an incremental index update.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateReport {
    /// Number of pages added or re-indexed.
    pub updated: usize,
    /// Number of pages removed from the index.
    pub deleted: usize,
}

/// Current health snapshot of a wiki's search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    /// Wiki name.
    pub wiki: String,
    /// Absolute path to the search-index directory.
    pub path: String,
    /// ISO-8601 timestamp of the last successful build, or None if never built.
    pub built: Option<String>,
    /// Number of pages in the index.
    pub pages: usize,
    /// Number of section pages in the index.
    pub sections: usize,
    /// True if the index is behind the current HEAD commit or schema.
    pub stale: bool,
    /// True if the index directory can be opened by Tantivy.
    pub openable: bool,
    /// True if the index can be queried (reader opened successfully).
    pub queryable: bool,
}

/// Classification of index staleness used to choose the cheapest rebuild strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StalenessKind {
    /// Index matches current HEAD and schema — no rebuild needed.
    Current,
    /// HEAD commit changed but schema is unchanged — incremental update sufficient.
    CommitChanged,
    /// Only specific type schemas changed — partial rebuild of those types sufficient.
    TypesChanged(Vec<String>),
    /// Schema changed in a way that requires a full rebuild.
    FullRebuildNeeded,
}

// ── state.toml ────────────────────────────────────────────────────────────────

/// Persisted state written to `state.toml` alongside the Tantivy index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexState {
    /// SHA-256 hash of the combined type registry, used for schema staleness detection.
    #[serde(default)]
    pub schema_hash: String,
    /// ISO-8601 timestamp of when the index was last successfully built.
    pub built: String,
    /// Number of pages in the index at last build.
    pub pages: usize,
    /// Number of section pages in the index at last build.
    pub sections: usize,
    /// Git HEAD commit hash at the time of the last build.
    pub commit: String,
    /// Per-type content hashes at last build (type name → hash).
    #[serde(default)]
    pub types: std::collections::HashMap<String, String>,
}

// ── SpaceIndexManager ─────────────────────────────────────────────────────────

struct IndexInner {
    tantivy_index: Option<Index>,
    index_reader: Option<IndexReader>,
    generation: AtomicU64,
}

/// Tantivy index lifecycle manager for a single wiki space.
pub struct SpaceIndexManager {
    wiki_name: String,
    index_path: PathBuf,
    inner: RwLock<IndexInner>,
}

impl SpaceIndexManager {
    /// Create a new `SpaceIndexManager` for `wiki_name` with its index stored at `index_path`.
    pub fn new(wiki_name: impl Into<String>, index_path: impl Into<PathBuf>) -> Self {
        Self {
            wiki_name: wiki_name.into(),
            index_path: index_path.into(),
            inner: RwLock::new(IndexInner {
                tantivy_index: None,
                index_reader: None,
                generation: AtomicU64::new(0),
            }),
        }
    }

    /// Return the absolute path to the index directory.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// Return the wiki name this manager is associated with.
    pub fn wiki_name(&self) -> &str {
        &self.wiki_name
    }

    /// Return the current generation counter value.
    /// Incremented on every successful `reload_reader()` call.
    pub fn generation(&self) -> u64 {
        self.inner
            .read()
            .unwrap()
            .generation
            .load(Ordering::Acquire)
    }

    /// Open the index from disk and hold the reader.
    /// Call after rebuild/staleness check. Recovery: if open fails and
    /// wiki_root/repo_root/registry are provided, rebuild and retry.
    pub fn open(
        &self,
        is: &IndexSchema,
        recovery: Option<(&Path, &Path, &SpaceTypeRegistry)>,
    ) -> Result<()> {
        let search_dir = self.index_path.join("search-index");

        let try_open = || -> Result<Index> {
            let dir = MmapDirectory::open(&search_dir)?;
            Ok(Index::open(dir)?)
        };

        let index = match try_open() {
            Ok(idx) => idx,
            Err(e) => {
                if let Some((wiki_root, repo_root, registry)) = recovery {
                    tracing::warn!(
                        wiki = %self.wiki_name,
                        error = %e,
                        "index corrupt, rebuilding",
                    );
                    if search_dir.exists() {
                        let _ = std::fs::remove_dir_all(&search_dir);
                    }
                    self.rebuild(wiki_root, repo_root, is, registry)?;
                    try_open().context("index still corrupt after rebuild")?
                } else {
                    return Err(e);
                }
            }
        };

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()?;
        let mut inner = self
            .inner
            .write()
            .map_err(|_| anyhow::anyhow!("index lock poisoned"))?;
        inner.tantivy_index = Some(index);
        inner.index_reader = Some(reader);
        Ok(())
    }

    /// Get a searcher. Cheap — arc clone of current segment set.
    pub fn searcher(&self) -> Result<Searcher> {
        let inner = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("index lock poisoned"))?;
        inner
            .index_reader
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("index not open"))
            .map(|r| r.searcher())
    }

    /// Reload the held IndexReader so searchers see the latest commit.
    /// No-op if the reader is not yet open. Safe to call after every write.
    fn reload_reader(&self) -> Result<()> {
        let inner = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("index lock poisoned"))?;
        if let Some(ref r) = inner.index_reader {
            r.reload()?;
        }
        inner.generation.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    /// Get a writer from the held index, or open from disk if not held.
    fn writer(&self) -> Result<IndexWriter> {
        let inner = self
            .inner
            .read()
            .map_err(|_| anyhow::anyhow!("index lock poisoned"))?;
        if let Some(ref idx) = inner.tantivy_index {
            Ok(idx.writer(50_000_000)?)
        } else {
            drop(inner);
            let search_dir = self.index_path.join("search-index");
            let dir = MmapDirectory::open(&search_dir)
                .with_context(|| format!("failed to open index dir: {}", search_dir.display()))?;
            let index = Index::open(dir).context("failed to open index")?;
            Ok(index.writer(50_000_000)?)
        }
    }

    /// Return the git commit hash recorded in `state.toml` at the last index build, if any.
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

    /// Rebuild the full index by walking all Markdown files under `wiki_root`.
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

        // Always open_or_create for rebuild (schema may have changed)
        let dir = MmapDirectory::open(&search_dir)
            .with_context(|| format!("failed to open index dir: {}", search_dir.display()))?;
        let index = Index::open_or_create(dir, is.schema.clone())?;
        let mut writer: IndexWriter = index.writer(50_000_000)?;
        writer.delete_all_documents()?;

        let mut pages = 0usize;
        let mut sections = 0usize;
        let mut skipped = 0usize;

        for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skipping unreadable file");
                    skipped += 1;
                    continue;
                }
            };

            let slug = match Slug::from_path(path, wiki_root) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skipping invalid path");
                    skipped += 1;
                    continue;
                }
            };
            let uri = format!("wiki://{}/{slug}", self.wiki_name);
            let page = frontmatter::parse(&content);

            writer.add_document(index_page(is, registry, slug.as_str(), &uri, &page))?;

            if page.page_type() == Some("section") {
                sections += 1;
            }
            pages += 1;
        }

        writer.commit()?;
        self.reload_reader()?;

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
            skipped,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Incrementally update the index for files changed since `last_indexed_commit`.
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

        let mut writer = self.writer()?;

        let f_slug = is.field("slug");
        let wiki_prefix = wiki_root
            .strip_prefix(repo_root)
            .unwrap_or(Path::new("wiki"));
        let mut updated = 0;
        let mut deleted = 0;

        for (path, status) in &changes {
            let slug = match Slug::from_path(path, wiki_prefix) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skipping invalid path in update");
                    continue;
                }
            };

            writer.delete_term(Term::from_field_text(f_slug, slug.as_str()));

            if *status == Delta::Deleted {
                deleted += 1;
            } else {
                let full_path = repo_root.join(path);
                if let Ok(content) = std::fs::read_to_string(&full_path) {
                    let page = frontmatter::parse(&content);
                    let uri = format!("wiki://{}/{slug}", self.wiki_name);
                    writer.add_document(index_page(is, registry, slug.as_str(), &uri, &page))?;
                    updated += 1;
                }
            }
        }

        writer.commit()?;
        self.reload_reader()?;
        Ok(UpdateReport { updated, deleted })
    }

    /// Return the current index health status (staleness, page count, openability).
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
                    let stale = state.commit != head || state.schema_hash != current_schema_hash;
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
                        .reader_builder()
                        .reload_policy(tantivy::ReloadPolicy::Manual)
                        .try_into()
                        .map(|r: IndexReader| {
                            r.searcher()
                                .search(&AllQuery, &TopDocs::with_limit(1).order_by_score())
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

    /// Delete all index documents whose `type` field equals `type_name`.
    pub fn delete_by_type(&self, is: &IndexSchema, type_name: &str) -> Result<()> {
        let mut writer = self.writer()?;
        let f_type = is.field("type");
        writer.delete_term(Term::from_field_text(f_type, type_name));
        writer.commit()?;
        self.reload_reader()?;
        Ok(())
    }

    /// Determine what kind of staleness exists.
    pub fn staleness_kind(&self, repo_root: &Path) -> Result<StalenessKind> {
        let state_path = self.index_path.join("state.toml");
        let state = match std::fs::read_to_string(&state_path)
            .ok()
            .and_then(|c| toml::from_str::<IndexState>(&c).ok())
        {
            Some(s) => s,
            None => return Ok(StalenessKind::FullRebuildNeeded),
        };

        let head = git::current_head(repo_root).unwrap_or_default();
        let (current_schema_hash, current_types) =
            crate::type_registry::compute_disk_hashes(repo_root).unwrap_or_default();

        if state.commit == head && state.schema_hash == current_schema_hash {
            return Ok(StalenessKind::Current);
        }

        if state.schema_hash == current_schema_hash {
            return Ok(StalenessKind::CommitChanged);
        }

        // Schema hash differs — check per-type
        let mut changed = Vec::new();
        for (name, hash) in &state.types {
            match current_types.get(name) {
                Some(h) if h != hash => changed.push(name.clone()),
                None => changed.push(name.clone()),
                _ => {}
            }
        }
        for name in current_types.keys() {
            if !state.types.contains_key(name) {
                changed.push(name.clone());
            }
        }

        if changed.is_empty() {
            Ok(StalenessKind::FullRebuildNeeded)
        } else {
            changed.sort();
            Ok(StalenessKind::TypesChanged(changed))
        }
    }

    /// Re-index only pages of the specified types.
    pub fn rebuild_types(
        &self,
        types: &[String],
        wiki_root: &Path,
        repo_root: &Path,
        is: &IndexSchema,
        registry: &SpaceTypeRegistry,
    ) -> Result<IndexReport> {
        let start = std::time::Instant::now();
        let mut writer = self.writer()?;
        let f_type = is.field("type");

        // Delete all documents of the changed types
        for type_name in types {
            writer.delete_term(Term::from_field_text(f_type, type_name));
        }

        // Re-index pages matching those types
        let type_set: std::collections::HashSet<&str> = types.iter().map(|s| s.as_str()).collect();
        let mut pages = 0usize;
        let mut skipped = 0usize;

        for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skipping unreadable file");
                    skipped += 1;
                    continue;
                }
            };
            let page = frontmatter::parse(&content);
            let page_type = page.page_type().unwrap_or("page");
            if !type_set.contains(page_type) {
                continue;
            }
            let slug = match Slug::from_path(path, wiki_root) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skipping invalid path");
                    skipped += 1;
                    continue;
                }
            };
            let uri = format!("wiki://{}/{slug}", self.wiki_name);
            writer.add_document(index_page(is, registry, slug.as_str(), &uri, &page))?;
            pages += 1;
        }

        writer.commit()?;
        self.reload_reader()?;

        // Update state.toml
        let commit = git::current_head(repo_root).unwrap_or_default();
        let state = IndexState {
            schema_hash: registry.schema_hash().to_string(),
            built: Utc::now().to_rfc3339(),
            pages: 0, // not accurate for partial, but state.toml is refreshed
            sections: 0,
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
            skipped,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// ── Document building (private) ───────────────────────────────────────────────

fn index_page(
    is: &IndexSchema,
    registry: &SpaceTypeRegistry,
    slug: &str,
    uri: &str,
    page: &frontmatter::ParsedPage,
) -> tantivy::TantivyDocument {
    let mut doc = tantivy::TantivyDocument::default();

    doc.add_text(is.field("slug"), slug);
    doc.add_text(is.field("uri"), uri);

    // Write confidence as f64 FAST field using the dedicated getter
    if let Some(conf_field) = is.try_field("confidence") {
        let conf = frontmatter::confidence(&page.frontmatter) as f64;
        doc.add_f64(conf_field, conf);
    }

    let resolved = resolve_fields(page, registry);
    let mut extra_text = String::new();

    for (canonical, value) in &resolved {
        // confidence is already written above as a numeric field; skip text indexing
        if canonical == "confidence" {
            continue;
        }
        index_value(&mut doc, &mut extra_text, is, canonical, value);
    }

    if extra_text.is_empty() {
        doc.add_text(is.field("body"), &page.body);
    } else {
        doc.add_text(
            is.field("body"),
            format!("{}\n{}", page.body, extra_text.trim()),
        );
    }

    for link in links::extract_body_wikilinks(&page.body) {
        doc.add_text(is.field("body_links"), &link);
    }

    doc
}

/// Resolve frontmatter fields through the type's alias map.
///
/// Two passes:
/// 1. Index non-aliased fields under their own name
/// 2. For aliased source fields, index under the canonical name
///    only if the canonical wasn't already present
fn resolve_fields<'a>(
    page: &'a frontmatter::ParsedPage,
    registry: &'a SpaceTypeRegistry,
) -> Vec<(String, &'a serde_yaml::Value)> {
    let page_type = page.page_type().unwrap_or("page");
    let empty = std::collections::HashMap::new();
    let aliases = registry.aliases(page_type).unwrap_or(&empty);

    let mut result = Vec::new();
    let mut indexed: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Pass 1: non-aliased fields
    for (field_name, value) in &page.frontmatter {
        if aliases.contains_key(field_name.as_str()) {
            continue;
        }
        let canonical = field_name.to_string();
        indexed.insert(canonical.clone());
        result.push((canonical, value));
    }

    // Pass 2: aliased source fields whose canonical target was not present
    for (source_field, canonical) in aliases {
        if indexed.contains(canonical.as_str()) {
            continue;
        }
        if let Some(value) = page.frontmatter.get(source_field.as_str()) {
            indexed.insert(canonical.clone());
            result.push((canonical.clone(), value));
        }
    }

    result
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
        serde_yaml::Value::Mapping(_) => serde_json::to_string(value).unwrap_or_default(),
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
