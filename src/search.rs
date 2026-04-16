use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, STORED, STRING},
    Index, IndexWriter, Snippet, SnippetGenerator, Term,
};
use walkdir::WalkDir;

use crate::frontmatter::parse_frontmatter;
use crate::git;
use crate::markdown::slug_for;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexReport {
    pub wiki: String,
    pub pages_indexed: usize,
    pub duration_ms: u64,
}

// ── state.toml ────────────────────────────────────────────────────────────────

const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexState {
    #[serde(default)]
    schema_version: u32,
    built: String,
    pages: usize,
    sections: usize,
    commit: String,
}

// ── Search options ────────────────────────────────────────────────────────────

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

// ── Recovery context ───────────────────────────────────────────────────────────

/// Optional context for auto-recovery on corrupt index.
pub struct RecoveryContext<'a> {
    pub wiki_root: &'a Path,
    pub repo_root: &'a Path,
}

// ── Tantivy schema ────────────────────────────────────────────────────────────

fn build_schema() -> Schema {
    let mut builder = Schema::builder();

    let text_indexing = TextFieldIndexing::default()
        .set_tokenizer("default")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_opts = TextOptions::default()
        .set_indexing_options(text_indexing)
        .set_stored();

    builder.add_text_field("slug", STRING | STORED);
    builder.add_text_field("title", text_opts.clone());
    builder.add_text_field("summary", text_opts.clone());
    builder.add_text_field("body", text_opts.clone());
    builder.add_text_field("type", STRING | STORED);
    builder.add_text_field("status", STRING | STORED);
    builder.add_text_field("tags", text_opts);
    builder.build()
}

/// Try to open the tantivy index. If it fails and recovery context is provided,
/// delete corrupt files, rebuild, and retry once.
fn open_index(
    search_dir: &Path,
    index_path: &Path,
    wiki_name: &str,
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
                    if let Err(e) = std::fs::remove_dir_all(search_dir) {
                        tracing::warn!(error = %e, "failed to delete corrupt index directory");
                    }
                }
                rebuild_index(ctx.wiki_root, index_path, wiki_name, ctx.repo_root)?;
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
) -> Result<IndexReport> {
    let start = std::time::Instant::now();
    let schema = build_schema();

    let search_dir = index_path.join("search-index");
    std::fs::create_dir_all(&search_dir)?;

    let dir = MmapDirectory::open(&search_dir)
        .with_context(|| format!("failed to open index dir: {}", search_dir.display()))?;
    let index = Index::open_or_create(dir, schema.clone())?;
    let mut writer: IndexWriter = index.writer(50_000_000)?;
    writer.delete_all_documents()?;

    let f_slug = schema.get_field("slug").unwrap();
    let f_title = schema.get_field("title").unwrap();
    let f_summary = schema.get_field("summary").unwrap();
    let f_body = schema.get_field("body").unwrap();
    let f_type = schema.get_field("type").unwrap();
    let f_status = schema.get_field("status").unwrap();
    let f_tags = schema.get_field("tags").unwrap();

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

        let slug = slug_for(path, wiki_root);

        let (fm, body) = match parse_frontmatter(&content) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let tags_str = fm.tags.join(" ");

        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(f_slug, &slug);
        doc.add_text(f_title, &fm.title);
        doc.add_text(f_summary, &fm.summary);
        doc.add_text(f_body, &body);
        doc.add_text(f_type, &fm.r#type);
        doc.add_text(f_status, &fm.status);
        doc.add_text(f_tags, &tags_str);
        writer.add_document(doc)?;

        if fm.r#type == "section" {
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
    let state_path = index_path.join("state.toml");
    std::fs::write(&state_path, toml::to_string_pretty(&state)?)?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(IndexReport {
        wiki: wiki_name.to_string(),
        pages_indexed: pages,
        duration_ms,
    })
}

// ── index_status ──────────────────────────────────────────────────────────────

pub fn index_status(wiki_name: &str, index_path: &Path, repo_root: &Path) -> Result<IndexStatus> {
    let state_path = index_path.join("state.toml");
    if !state_path.exists() {
        return Ok(IndexStatus {
            wiki: wiki_name.to_string(),
            path: index_path.join("search-index").to_string_lossy().into(),
            built: None,
            pages: 0,
            sections: 0,
            stale: true,
        });
    }

    let content = match std::fs::read_to_string(&state_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(wiki = %wiki_name, error = %e, "failed to read state.toml");
            return Ok(IndexStatus {
                wiki: wiki_name.to_string(),
                path: index_path.join("search-index").to_string_lossy().into(),
                built: None,
                pages: 0,
                sections: 0,
                stale: true,
            });
        }
    };
    let state: IndexState = match toml::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(wiki = %wiki_name, error = %e, "malformed state.toml");
            return Ok(IndexStatus {
                wiki: wiki_name.to_string(),
                path: index_path.join("search-index").to_string_lossy().into(),
                built: None,
                pages: 0,
                sections: 0,
                stale: true,
            });
        }
    };

    let head = git::current_head(repo_root).unwrap_or_default();
    let stale = state.commit != head || state.schema_version != CURRENT_SCHEMA_VERSION;

    Ok(IndexStatus {
        wiki: wiki_name.to_string(),
        path: index_path.join("search-index").to_string_lossy().into(),
        built: Some(state.built),
        pages: state.pages,
        sections: state.sections,
        stale,
    })
}

// ── search ────────────────────────────────────────────────────────────────────

pub fn search(
    query_str: &str,
    options: &SearchOptions,
    index_path: &Path,
    wiki_name: &str,
    recovery: Option<&RecoveryContext<'_>>,
) -> Result<Vec<PageRef>> {
    let search_dir = index_path.join("search-index");
    if !search_dir.exists() {
        bail!("search index not found — run `wiki index rebuild`");
    }

    let schema = build_schema();
    let index = open_index(&search_dir, index_path, wiki_name, recovery)?;
    let reader = index.reader()?;
    let searcher = reader.searcher();

    let f_slug = schema.get_field("slug").unwrap();
    let f_title = schema.get_field("title").unwrap();
    let f_summary = schema.get_field("summary").unwrap();
    let f_body = schema.get_field("body").unwrap();
    let f_type = schema.get_field("type").unwrap();

    let query_parser = QueryParser::for_index(&index, vec![f_title, f_summary, f_body]);
    let parsed = query_parser
        .parse_query(query_str)
        .with_context(|| format!("failed to parse query: {query_str}"))?;

    // Build final query with optional filters
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
            let text = snippet.to_html();
            if text.is_empty() {
                String::new()
            } else {
                text
            }
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
    recovery: Option<&RecoveryContext<'_>>,
) -> Result<PageList> {
    let search_dir = index_path.join("search-index");
    if !search_dir.exists() {
        bail!("search index not found — run `wiki index rebuild`");
    }

    let schema = build_schema();
    let index = open_index(&search_dir, index_path, wiki_name, recovery)?;
    let reader = index.reader()?;
    let searcher = reader.searcher();

    let f_slug = schema.get_field("slug").unwrap();
    let f_title = schema.get_field("title").unwrap();
    let f_type = schema.get_field("type").unwrap();
    let f_status = schema.get_field("status").unwrap();
    let f_tags = schema.get_field("tags").unwrap();

    // Build filter query
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

    // Collect all matching docs
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

    // Sort by slug
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

/// Search across multiple wikis, merge results by score descending, truncate to top_k.
/// Each entry is (wiki_name, index_path). Wikis without an index are silently skipped.
pub fn search_all(
    query_str: &str,
    options: &SearchOptions,
    wikis: &[(String, PathBuf)],
) -> Result<Vec<PageRef>> {
    let mut all_results = Vec::new();
    for (name, index_path) in wikis {
        match search(query_str, options, index_path, name, None) {
            Ok(results) => all_results.extend(results),
            Err(_) => continue, // skip wikis without index
        }
    }
    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all_results.truncate(options.top_k);
    Ok(all_results)
}
