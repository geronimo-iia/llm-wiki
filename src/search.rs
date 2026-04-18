use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use git2::Delta;
use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Value},
    Index, IndexWriter, Snippet, SnippetGenerator, Term,
};
use walkdir::WalkDir;

use crate::frontmatter;
use crate::git;
use crate::index_schema::IndexSchema;
use crate::links;
use crate::slug::Slug;

// ── Return types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRef {
    pub slug: String,
    pub uri: String,
    pub title: String,
    pub score: f32,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    pub slug: String,
    pub uri: String,
    pub title: String,
    pub r#type: String,
    pub status: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageList {
    pub pages: Vec<PageSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
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

// ── state.toml ────────────────────────────────────────────────────────────────

const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexState {
    #[serde(default)]
    schema_version: u32,
    built: String,
    pages: usize,
    sections: usize,
    commit: String,
}

pub fn last_indexed_commit(index_path: &Path) -> Option<String> {
    let state_path = index_path.join("state.toml");
    let content = std::fs::read_to_string(&state_path).ok()?;
    let state: IndexState = toml::from_str(&content).ok()?;
    if state.commit.is_empty() {
        None
    } else {
        Some(state.commit)
    }
}

// ── Options ───────────────────────────────────────────────────────────────────

pub struct SearchOptions {
    pub no_excerpt: bool,
    pub include_sections: bool,
    pub top_k: usize,
    pub r#type: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            no_excerpt: false,
            include_sections: false,
            top_k: 10,
            r#type: None,
        }
    }
}

pub struct ListOptions {
    pub r#type: Option<String>,
    pub status: Option<String>,
    pub page: usize,
    pub page_size: usize,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            r#type: None,
            status: None,
            page: 1,
            page_size: 20,
        }
    }
}

/// Optional context for auto-recovery on corrupt index.
pub struct RecoveryContext<'a> {
    pub wiki_root: &'a Path,
    pub repo_root: &'a Path,
}

// ── Document building ─────────────────────────────────────────────────────────

fn build_document(
    is: &IndexSchema,
    slug: &str,
    uri: &str,
    page: &frontmatter::ParsedPage,
) -> tantivy::TantivyDocument {
    let mut doc = tantivy::TantivyDocument::default();
    doc.add_text(is.field("slug"), slug);
    doc.add_text(is.field("uri"), uri);
    doc.add_text(is.field("title"), page.title().unwrap_or(""));
    doc.add_text(
        is.field("summary"),
        page.frontmatter
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    doc.add_text(is.field("body"), &page.body);
    doc.add_text(is.field("type"), page.page_type().unwrap_or("page"));
    doc.add_text(is.field("status"), page.status().unwrap_or("active"));
    doc.add_text(is.field("tags"), page.tags().join(" "));

    // Body wiki-links as multi-valued keyword field
    for link in links::extract_body_wikilinks(&page.body) {
        doc.add_text(is.field("body_links"), &link);
    }

    doc
}

// ── Index open with recovery ──────────────────────────────────────────────────

fn open_index(
    search_dir: &Path,
    index_path: &Path,
    wiki_name: &str,
    is: &IndexSchema,
    recovery: Option<&RecoveryContext<'_>>,
) -> Result<Index> {
    let try_open = || -> Result<Index> {
        let dir = MmapDirectory::open(search_dir)?;
        Ok(Index::open(dir)?)
    };

    match try_open() {
        Ok(idx) => Ok(idx),
        Err(e) => {
            if let Some(ctx) = recovery {
                tracing::warn!(
                    wiki = %wiki_name,
                    error = %e,
                    "index corrupt, rebuilding",
                );
                if search_dir.exists() {
                    let _ = std::fs::remove_dir_all(search_dir);
                }
                rebuild_index(ctx.wiki_root, index_path, wiki_name, ctx.repo_root, is)?;
                try_open().context("index still corrupt after rebuild")
            } else {
                Err(e)
            }
        }
    }
}

// ── rebuild_index ─────────────────────────────────────────────────────────────

pub fn rebuild_index(
    wiki_root: &Path,
    index_path: &Path,
    wiki_name: &str,
    repo_root: &Path,
    is: &IndexSchema,
) -> Result<IndexReport> {
    let start = std::time::Instant::now();

    let search_dir = index_path.join("search-index");
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
        let uri = format!("wiki://{wiki_name}/{slug}");
        let page = frontmatter::parse(&content);

        writer.add_document(build_document(is, slug.as_str(), &uri, &page))?;

        if page.page_type() == Some("section") {
            sections += 1;
        }
        pages += 1;
    }

    writer.commit()?;

    let commit = git::current_head(repo_root).unwrap_or_default();
    let state = IndexState {
        schema_version: CURRENT_SCHEMA_VERSION,
        built: Utc::now().to_rfc3339(),
        pages,
        sections,
        commit,
    };
    std::fs::write(
        index_path.join("state.toml"),
        toml::to_string_pretty(&state)?,
    )?;

    Ok(IndexReport {
        wiki: wiki_name.to_string(),
        pages_indexed: pages,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

// ── collect_changed_files ─────────────────────────────────────────────────────

pub fn collect_changed_files(
    repo_root: &Path,
    wiki_root: &Path,
    last_indexed_commit: Option<&str>,
) -> Result<HashMap<PathBuf, Delta>> {
    let mut changes = HashMap::new();

    // B: last indexed commit vs HEAD (insert first so A wins on duplicates)
    if let Some(from_hash) = last_indexed_commit {
        if let Ok(files) = git::changed_since_commit(repo_root, wiki_root, from_hash) {
            for f in files {
                changes.insert(f.path, f.status);
            }
        }
    }

    // A: working tree vs HEAD (overwrites B on duplicates)
    if let Ok(files) = git::changed_wiki_files(repo_root, wiki_root) {
        for f in files {
            changes.insert(f.path, f.status);
        }
    }

    Ok(changes)
}

// ── update_index ──────────────────────────────────────────────────────────────

pub fn update_index(
    wiki_root: &Path,
    index_path: &Path,
    repo_root: &Path,
    last_indexed_commit: Option<&str>,
    is: &IndexSchema,
    wiki_name: &str,
) -> Result<UpdateReport> {
    let changes = collect_changed_files(repo_root, wiki_root, last_indexed_commit)?;
    if changes.is_empty() {
        return Ok(UpdateReport::default());
    }

    let search_dir = index_path.join("search-index");
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
                let uri = format!("wiki://{wiki_name}/{slug}");
                writer.add_document(build_document(is, slug.as_str(), &uri, &page))?;
                updated += 1;
            }
        }
    }

    writer.commit()?;
    Ok(UpdateReport { updated, deleted })
}

// ── index_status (merged with index_check) ────────────────────────────────────

pub fn index_status(wiki_name: &str, index_path: &Path, repo_root: &Path) -> Result<IndexStatus> {
    let state_path = index_path.join("state.toml");
    let search_dir = index_path.join("search-index");

    let (built, pages, sections, stale) = if state_path.exists() {
        match std::fs::read_to_string(&state_path)
            .ok()
            .and_then(|c| toml::from_str::<IndexState>(&c).ok())
        {
            Some(state) => {
                let head = git::current_head(repo_root).unwrap_or_default();
                let stale =
                    state.commit != head || state.schema_version != CURRENT_SCHEMA_VERSION;
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
        wiki: wiki_name.to_string(),
        path: search_dir.to_string_lossy().into(),
        built,
        pages,
        sections,
        stale,
        openable,
        queryable,
    })
}

// ── search ────────────────────────────────────────────────────────────────────

pub fn search(
    query_str: &str,
    options: &SearchOptions,
    index_path: &Path,
    wiki_name: &str,
    is: &IndexSchema,
    recovery: Option<&RecoveryContext<'_>>,
) -> Result<Vec<PageRef>> {
    let search_dir = index_path.join("search-index");
    if !search_dir.exists() {
        bail!("search index not found — run `llm-wiki index rebuild`");
    }

    let index = open_index(&search_dir, index_path, wiki_name, is, recovery)?;
    let reader = index.reader()?;
    let searcher = reader.searcher();

    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_summary = is.field("summary");
    let f_body = is.field("body");
    let f_type = is.field("type");

    let query_parser = QueryParser::for_index(&index, vec![f_title, f_summary, f_body]);
    let parsed = query_parser
        .parse_query(query_str)
        .with_context(|| format!("failed to parse query: {query_str}"))?;

    let final_query: Box<dyn tantivy::query::Query> = {
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
        clauses.push((Occur::Must, parsed));

        if !options.include_sections {
            clauses.push((
                Occur::MustNot,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, "section"),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some(ref type_filter) = options.r#type {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, type_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        Box::new(BooleanQuery::new(clauses))
    };

    let top_docs = searcher.search(&final_query, &TopDocs::with_limit(options.top_k))?;

    let snippet_gen = if !options.no_excerpt {
        Some(SnippetGenerator::create(&searcher, &final_query, f_body)?)
    } else {
        None
    };

    let mut results = Vec::new();
    for (score, doc_addr) in top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(doc_addr)?;

        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = doc
            .get_first(f_title)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let uri = format!("wiki://{wiki_name}/{slug}");

        let excerpt = snippet_gen.as_ref().map(|gen| {
            let snippet: Snippet = gen.snippet_from_doc(&doc);
            snippet.to_html()
        });

        results.push(PageRef {
            slug,
            uri,
            title,
            score,
            excerpt,
        });
    }

    Ok(results)
}

// ── list ──────────────────────────────────────────────────────────────────────

pub fn list(
    options: &ListOptions,
    index_path: &Path,
    wiki_name: &str,
    is: &IndexSchema,
    recovery: Option<&RecoveryContext<'_>>,
) -> Result<PageList> {
    let search_dir = index_path.join("search-index");
    if !search_dir.exists() {
        bail!("search index not found — run `llm-wiki index rebuild`");
    }

    let index = open_index(&search_dir, index_path, wiki_name, is, recovery)?;
    let reader = index.reader()?;
    let searcher = reader.searcher();

    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_type = is.field("type");
    let f_status = is.field("status");
    let f_tags = is.field("tags");

    let query: Box<dyn tantivy::query::Query> = {
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        if let Some(ref type_filter) = options.r#type {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, type_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some(ref status_filter) = options.status {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_status, status_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if clauses.is_empty() {
            Box::new(AllQuery)
        } else {
            Box::new(BooleanQuery::new(clauses))
        }
    };

    let top_docs = searcher.search(&query, &TopDocs::with_limit(100_000))?;

    let mut summaries: Vec<PageSummary> = Vec::new();
    for (_score, doc_addr) in top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(doc_addr)?;

        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = doc
            .get_first(f_title)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let status = doc
            .get_first(f_status)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tags_str = doc
            .get_first(f_tags)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tags: Vec<String> = tags_str
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let uri = format!("wiki://{wiki_name}/{slug}");

        summaries.push(PageSummary {
            slug,
            uri,
            title,
            r#type: page_type,
            status,
            tags,
        });
    }

    summaries.sort_by(|a, b| a.slug.cmp(&b.slug));

    let total = summaries.len();
    let page = options.page;
    let page_size = options.page_size;
    let start = (page - 1) * page_size;
    let pages = if start < total {
        summaries[start..(start + page_size).min(total)].to_vec()
    } else {
        Vec::new()
    };

    Ok(PageList {
        pages,
        total,
        page,
        page_size,
    })
}

// ── search_all ────────────────────────────────────────────────────────────────

pub fn search_all(
    query_str: &str,
    options: &SearchOptions,
    wikis: &[(String, PathBuf)],
    is: &IndexSchema,
) -> Result<Vec<PageRef>> {
    let mut all_results = Vec::new();
    for (name, index_path) in wikis {
        match search(query_str, options, index_path, name, is, None) {
            Ok(results) => all_results.extend(results),
            Err(_) => continue,
        }
    }
    all_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.top_k);
    Ok(all_results)
}
