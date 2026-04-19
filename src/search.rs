use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Value},
    Snippet, SnippetGenerator, Term,
};

use crate::index_schema::IndexSchema;
use crate::indexing;

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

// ── search ────────────────────────────────────────────────────────────────────

pub fn search(
    query_str: &str,
    options: &SearchOptions,
    index_path: &Path,
    wiki_name: &str,
    is: &IndexSchema,
    recovery: Option<&indexing::RecoveryContext<'_>>,
) -> Result<Vec<PageRef>> {
    let search_dir = index_path.join("search-index");
    if !search_dir.exists() {
        bail!("search index not found — run `llm-wiki index rebuild`");
    }

    let index = indexing::open_index(&search_dir, index_path, wiki_name, is, recovery)?;
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
    recovery: Option<&indexing::RecoveryContext<'_>>,
) -> Result<PageList> {
    let search_dir = index_path.join("search-index");
    if !search_dir.exists() {
        bail!("search index not found — run `llm-wiki index rebuild`");
    }

    let index = indexing::open_index(&search_dir, index_path, wiki_name, is, recovery)?;
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
