//! Context assembly — retrieve the top-K most relevant wiki pages for a question
//! and format them as Markdown for an external LLM to synthesise from.
//!
//! ## Output format
//!
//! Each page is rendered as:
//!
//! ```text
//! # {title}
//!
//! {full page body}
//!
//! ---
//!
//! ```
//!
//! Multiple pages are concatenated in BM25 relevance order. Contradiction pages
//! are not filtered — they are high-value context that captures the *structure*
//! of a knowledge domain.
//!
//! ## Empty output
//!
//! If the question matches no pages, or if all matched pages fail to load,
//! an empty string is returned (not an error).

use anyhow::Result;
use std::path::Path;

use crate::markdown::{parse_frontmatter, resolve_slug};
use crate::search::search;

/// Return the top-`top_k` wiki pages relevant to `question` as a single
/// Markdown string.
///
/// Internally runs a BM25 tantivy query against the wiki and includes any
/// contradiction pages that surface in the results — these are context gold
/// for an LLM synthesising an answer.
///
/// Returns an empty string (not an error) when no pages match.
pub fn context(question: &str, wiki_root: &Path, top_k: usize) -> Result<String> {
    if question.trim().is_empty() {
        return Ok(String::new());
    }

    let results = search(question, wiki_root, false)?;

    let mut output = String::new();
    for result in results.into_iter().take(top_k) {
        let page_path = match resolve_slug(wiki_root, &result.slug) {
            Some(p) => p,
            None => continue,
        };

        let content = match std::fs::read_to_string(&page_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (_fm, body) = match parse_frontmatter(&content) {
            Ok(r) => r,
            Err(_) => continue,
        };

        output.push_str(&format!("# {}\n\n{}\n---\n\n", result.title, body));
    }

    Ok(output)
}
