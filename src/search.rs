//! Full-text search via tantivy.
//!
//! The tantivy index lives in `.wiki/search-index/` (gitignored) and is
//! managed incrementally.
//!
//! ## Index lifecycle
//!
//! - `open_or_build_index` opens an existing index or builds one from scratch.
//! - `update_index` deletes and re-indexes only the changed slugs.
//! - `build_index` wipes any existing index and creates a fresh one — called
//!   only by `wiki search --rebuild-index`.

use anyhow::{Context, Result};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, STRING, TEXT};
use tantivy::{Index, IndexWriter, TantivyDocument, Term};
use walkdir::WalkDir;

use crate::markdown::{parse_frontmatter, slug_for};
use crate::registry::WikiRegistry;

/// A single search result returned by [`search`].
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Page slug — relative path from wiki root without the `.md` extension.
    pub slug: String,
    /// Absolute path to the page file.
    pub path: String,
    /// Page title from frontmatter.
    pub title: String,
    /// First ~200 characters of the page body — a quick reading preview.
    pub snippet: String,
    /// BM25 relevance score (higher is more relevant).
    pub score: f32,
    /// Page category (`concept`, `source-summary`, `query-result`, `contradiction`).
    pub page_type: String,
}

/// Build the tantivy schema used for all wiki indexes.
///
/// Fields:
/// - `slug`  — STRING | STORED  (exact token, stored for retrieval)
/// - `path`  — STRING | STORED  (absolute file path, stored)
/// - `title` — TEXT  | STORED  (tokenised full-text, stored)
/// - `tags`  — TEXT  | STORED  (space-joined tag list, tokenised)
/// - `body`  — TEXT  | STORED  (page body, tokenised; stored for snippet)
/// - `type`  — STRING | STORED  (exact page category, stored)
fn wiki_schema() -> Schema {
    let mut b = Schema::builder();
    b.add_text_field("slug", STRING | STORED);
    b.add_text_field("path", STRING | STORED);
    b.add_text_field("title", TEXT | STORED);
    b.add_text_field("tags", TEXT | STORED);
    b.add_text_field("body", TEXT | STORED);
    b.add_text_field("type", STRING | STORED);
    b.build()
}

/// Build (or rebuild) the tantivy index from all `.md` files under `wiki_root`.
///
/// Any existing index at `index_dir` is wiped first. Files inside `raw/` and
/// `.wiki/` are skipped. Pages that fail frontmatter parsing are skipped
/// silently (they may be raw Markdown not produced by the wiki engine).
///
/// Returns the opened [`Index`] ready for querying.
pub fn build_index(wiki_root: &Path, index_dir: &Path) -> Result<Index> {
    // Wipe and recreate the index directory.
    if index_dir.exists() {
        std::fs::remove_dir_all(index_dir)
            .with_context(|| format!("failed to remove index dir {}", index_dir.display()))?;
    }
    std::fs::create_dir_all(index_dir)
        .with_context(|| format!("failed to create index dir {}", index_dir.display()))?;

    let schema = wiki_schema();
    let index = Index::create_in_dir(index_dir, schema.clone())
        .context("failed to create tantivy index")?;
    let mut writer: IndexWriter = index
        .writer(50_000_000)
        .context("failed to create index writer")?;

    let slug_field = schema
        .get_field("slug")
        .context("slug field missing from schema")?;
    let path_field = schema
        .get_field("path")
        .context("path field missing from schema")?;
    let title_field = schema
        .get_field("title")
        .context("title field missing from schema")?;
    let tags_field = schema
        .get_field("tags")
        .context("tags field missing from schema")?;
    let body_field = schema
        .get_field("body")
        .context("body field missing from schema")?;
    let type_field = schema
        .get_field("type")
        .context("type field missing from schema")?;

    let raw_dir = wiki_root.join("raw");
    let wiki_dir = wiki_root.join(".wiki");
    let assets_dir = wiki_root.join("assets");
    let assets_index = assets_dir.join("index.md");

    for entry in WalkDir::new(wiki_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only index `.md` files.
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // Skip raw/ and .wiki/.
        if path.starts_with(&raw_dir) || path.starts_with(&wiki_dir) {
            continue;
        }

        // Skip assets/ subtree except assets/index.md.
        if path.starts_with(&assets_dir) && path != assets_index {
            continue;
        }

        // Skip non-index.md files inside bundle folders.
        // A bundle folder is any directory that contains an index.md.
        // Non-index .md files inside such a folder are assets, not pages.
        if let Some(filename) = path.file_name() {
            if filename != "index.md" {
                if let Some(parent) = path.parent() {
                    if parent.join("index.md").exists() {
                        continue;
                    }
                }
            }
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (fm, body) = match parse_frontmatter(&content) {
            Ok(r) => r,
            Err(_) => continue, // not a wiki-managed page; skip
        };

        let slug = slug_for(path, wiki_root);
        let abs_path = path.to_string_lossy().into_owned();

        let tags_str = fm.tags.join(" ");
        let type_str = serde_json::to_string(&fm.page_type)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let mut doc = TantivyDocument::default();
        doc.add_text(slug_field, &slug);
        doc.add_text(path_field, &abs_path);
        doc.add_text(title_field, &fm.title);
        doc.add_text(tags_field, &tags_str);
        doc.add_text(body_field, body);
        doc.add_text(type_field, &type_str);

        writer
            .add_document(doc)
            .context("failed to add document to index")?;
    }

    writer.commit().context("failed to commit index")?;
    Ok(index)
}

/// Open an existing tantivy index at `index_dir`, or build one from scratch
/// if the directory is missing or the index is corrupt.
pub fn open_or_build_index(wiki_root: &Path, index_dir: &Path) -> Result<Index> {
    if index_dir.exists() {
        if let Ok(index) = Index::open_in_dir(index_dir) {
            return Ok(index);
        }
    }
    build_index(wiki_root, index_dir)
}

/// Update the index incrementally for the given `changed_slugs`.
///
/// For each slug: deletes any existing document, then re-indexes from disk
/// if the file still exists. Slugs that no longer exist are deleted only.
/// Commits the writer. Does nothing if `changed_slugs` is empty.
pub fn update_index(wiki_root: &Path, index_dir: &Path, changed_slugs: &[String]) -> Result<()> {
    if changed_slugs.is_empty() {
        return Ok(());
    }

    let index = open_or_build_index(wiki_root, index_dir)?;
    let schema = index.schema();
    let slug_field = schema.get_field("slug").context("slug field missing")?;
    let path_field = schema.get_field("path").context("path field missing")?;
    let title_field = schema.get_field("title").context("title field missing")?;
    let tags_field = schema.get_field("tags").context("tags field missing")?;
    let body_field = schema.get_field("body").context("body field missing")?;
    let type_field = schema.get_field("type").context("type field missing")?;

    let mut writer: IndexWriter = index.writer(50_000_000).context("failed to create writer")?;

    for slug in changed_slugs {
        // Delete existing document for this slug.
        let term = Term::from_field_text(slug_field, slug);
        writer.delete_term(term);

        // Re-index if the file still exists on disk (flat or bundle).
        let path = match crate::markdown::resolve_slug(wiki_root, slug) {
            Some(p) => p,
            None => continue,
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let (fm, body) = match parse_frontmatter(&content) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let abs_path = path.to_string_lossy().into_owned();
        let tags_str = fm.tags.join(" ");
        let type_str = serde_json::to_string(&fm.page_type)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        let mut doc = TantivyDocument::default();
        doc.add_text(slug_field, slug);
        doc.add_text(path_field, &abs_path);
        doc.add_text(title_field, &fm.title);
        doc.add_text(tags_field, &tags_str);
        doc.add_text(body_field, body);
        doc.add_text(type_field, &type_str);
        writer.add_document(doc).context("failed to add document")?;
    }

    writer.commit().context("failed to commit index")?;
    Ok(())
}

/// Query a tantivy [`Index`] and return BM25-ranked [`SearchResult`]s.
///
/// Searches `title`, `tags`, and `body` fields. Returns at most `limit` results.
/// Returns an empty slice (not an error) if the query string cannot be parsed or
/// if no documents match.
pub fn search_index(index: &Index, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let schema = index.schema();
    let slug_field = schema
        .get_field("slug")
        .context("slug field missing from schema")?;
    let path_field = schema
        .get_field("path")
        .context("path field missing from schema")?;
    let title_field = schema
        .get_field("title")
        .context("title field missing from schema")?;
    let tags_field = schema
        .get_field("tags")
        .context("tags field missing from schema")?;
    let body_field = schema
        .get_field("body")
        .context("body field missing from schema")?;
    let type_field = schema
        .get_field("type")
        .context("type field missing from schema")?;

    let reader = index
        .reader()
        .context("failed to open index reader")?;
    let searcher = reader.searcher();

    let query_parser =
        QueryParser::for_index(index, vec![title_field, tags_field, body_field]);
    let query = match query_parser.parse_query(query_str) {
        Ok(q) => q,
        Err(_) => return Ok(Vec::new()),
    };

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(limit))
        .context("search failed")?;

    let mut results = Vec::with_capacity(top_docs.len());
    for (score, doc_addr) in top_docs {
        let doc: TantivyDocument = searcher.doc(doc_addr).context("failed to retrieve doc")?;

        let slug = doc
            .get_first(slug_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let path = doc
            .get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = doc
            .get_first(title_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let snippet = doc
            .get_first(body_field)
            .and_then(|v| v.as_str())
            .map(|b: &str| b.chars().take(200).collect::<String>())
            .unwrap_or_default();
        let page_type = doc
            .get_first(type_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        results.push(SearchResult {
            slug,
            path,
            title,
            snippet,
            score,
            page_type,
        });
    }

    Ok(results)
}

/// A search result that includes the source wiki name, for `wiki search --all`.
#[derive(Debug, Clone)]
pub struct SearchResultWithWiki {
    /// Name of the wiki that contains this result.
    pub wiki_name: String,
    /// Page slug — relative path from the wiki root without the `.md` extension.
    pub slug: String,
    /// Absolute path to the page file.
    pub path: String,
    /// Page title from frontmatter.
    pub title: String,
    /// First ~200 characters of the page body.
    pub snippet: String,
    /// BM25 relevance score.
    pub score: f32,
    /// Page category (`concept`, `source-summary`, …).
    pub page_type: String,
}

/// Fan out a query to every registered wiki, merge all results by descending
/// BM25 score, and truncate to `limit`.
///
/// Wikis that have no pages or an un-buildable index are silently skipped.
pub fn search_all(
    registry: &WikiRegistry,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResultWithWiki>> {
    let mut all: Vec<SearchResultWithWiki> = Vec::new();

    for entry in registry.entries() {
        let index_dir = entry.path.join(".wiki").join("search-index");
        let index = match open_or_build_index(&entry.path, &index_dir) {
            Ok(i) => i,
            Err(_) => continue,
        };
        let results = search_index(&index, query, limit).unwrap_or_default();
        for r in results {
            all.push(SearchResultWithWiki {
                wiki_name: entry.name.clone(),
                slug: r.slug,
                path: r.path,
                title: r.title,
                snippet: r.snippet,
                score: r.score,
                page_type: r.page_type,
            });
        }
    }

    all.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all.truncate(limit);
    Ok(all)
}

/// Search the wiki using `query` and return BM25-ranked results.
///
/// Opens the existing index if present, or builds it on first use.
/// Pass `rebuild_index = true` to force a full rebuild before querying.
pub fn search(query: &str, wiki_root: &Path, rebuild_index: bool) -> Result<Vec<SearchResult>> {
    let index_dir = wiki_root.join(".wiki").join("search-index");
    let index = if rebuild_index {
        build_index(wiki_root, &index_dir)?
    } else {
        open_or_build_index(wiki_root, &index_dir)?
    };
    search_index(&index, query, 20)
}
