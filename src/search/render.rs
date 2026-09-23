use super::types::{PageList, PageSummary, SearchResult};

// ── llms renderers ────────────────────────────────────────────────────────────

/// Render a `PageList` as LLM-optimized markdown: pages grouped by type,
/// one line per page with summary. Archived pages shown with strikethrough.
pub fn render_list_llms(result: &PageList) -> String {
    // Group by type, sorted by count desc then name asc
    let mut by_type: std::collections::HashMap<String, Vec<&PageSummary>> =
        std::collections::HashMap::new();
    for page in &result.pages {
        by_type.entry(page.r#type.clone()).or_default().push(page);
    }
    let mut groups: Vec<(String, Vec<&PageSummary>)> = by_type.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));

    let mut out = String::new();
    for (type_name, mut pages) in groups {
        pages.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.title.cmp(&b.title))
        });
        out.push_str(&format!("## {} ({})\n\n", type_name, pages.len()));
        for page in pages {
            let summary = page.summary.as_deref().unwrap_or("");
            let line = if page.status == "archived" {
                if summary.is_empty() {
                    format!("- ~~[{}]({})~~\n", page.title, page.uri)
                } else {
                    format!("- ~~[{}]({}): {}~~\n", page.title, page.uri, summary)
                }
            } else if summary.is_empty() {
                format!("- [{}]({})\n", page.title, page.uri)
            } else {
                format!("- [{}]({}): {}\n", page.title, page.uri, summary)
            };
            out.push_str(&line);
        }
        out.push('\n');
    }

    if result.total > result.page_size {
        let total_pages = (result.total + result.page_size - 1) / result.page_size.max(1);
        out.push_str(&format!(
            "_Page {}/{} — {} total pages_\n",
            result.page, total_pages, result.total
        ));
    }

    out
}

/// Render a `SearchResult` as LLM-optimized markdown: one line per result
/// with title, uri, and summary. No score, no excerpt block.
pub fn render_search_llms(result: &SearchResult) -> String {
    if result.results.is_empty() {
        return "No results found.\n".to_string();
    }
    let mut out = String::new();
    for r in &result.results {
        let summary = r.summary.as_deref().unwrap_or("");
        if summary.is_empty() {
            out.push_str(&format!("- [{}]({})\n", r.title, r.uri));
        } else {
            out.push_str(&format!("- [{}]({}): {}\n", r.title, r.uri, summary));
        }
    }
    out
}
