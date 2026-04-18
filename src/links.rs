use std::collections::HashSet;

use crate::frontmatter::ParsedPage;

/// Extract all linked slugs from a parsed page: frontmatter `sources`,
/// `concepts`, and body `[[wikilinks]]`. Deduplicated, order preserved.
pub fn extract_links(page: &ParsedPage) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for slug in page.string_list("sources") {
        if seen.insert(slug.to_string()) {
            result.push(slug.to_string());
        }
    }
    for slug in page.string_list("concepts") {
        if seen.insert(slug.to_string()) {
            result.push(slug.to_string());
        }
    }
    extract_wikilinks(&page.body, &mut seen, &mut result);

    result
}

/// Extract `[[slug]]` patterns from body text.
pub fn extract_wikilinks(text: &str, seen: &mut HashSet<String>, result: &mut Vec<String>) {
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let slug = after[..end].trim();
            if !slug.is_empty() && seen.insert(slug.to_string()) {
                result.push(slug.to_string());
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
}

/// Extract only body `[[wikilinks]]` from raw text (no frontmatter parsing).
pub fn extract_body_wikilinks(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    extract_wikilinks(text, &mut seen, &mut result);
    result
}
