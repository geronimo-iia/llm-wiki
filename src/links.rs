use std::collections::HashSet;

use crate::frontmatter::ParsedPage;

// ── ParsedLink ────────────────────────────────────────────────────────────────

/// A link value from a frontmatter edge field or body `[[wikilink]]`, classified by scope.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedLink {
    /// Bare slug resolved within the current wiki.
    Local(String),
    /// `wiki://name/slug` URI resolved in another mounted wiki.
    CrossWiki {
        /// Name of the target wiki in the `wiki://` URI.
        wiki: String,
        /// Slug within the target wiki.
        slug: String,
    },
}

impl ParsedLink {
    /// Parse a raw link string into a `ParsedLink`, classifying `wiki://` URIs as `CrossWiki`.
    pub fn parse(s: &str) -> Self {
        if let Some(rest) = s.strip_prefix("wiki://")
            && let Some(slash) = rest.find('/')
        {
            return ParsedLink::CrossWiki {
                wiki: rest[..slash].to_string(),
                slug: rest[slash + 1..].to_string(),
            };
        }
        ParsedLink::Local(s.to_string())
    }

    /// Return the slug portion of the link (local slug, or the slug segment of a cross-wiki URI).
    pub fn as_raw(&self) -> &str {
        match self {
            ParsedLink::Local(s) => s,
            ParsedLink::CrossWiki { wiki, slug } => {
                // We store the original string form; callers needing the raw
                // form reconstruct it. This returns the slug portion only for
                // local use; graph.rs uses the wiki/slug fields directly.
                let _ = wiki;
                slug
            }
        }
    }
}

/// Like `extract_links` but returns typed `ParsedLink` values distinguishing
/// local slugs from `wiki://name/slug` cross-wiki references.
/// Use this in graph.rs. The original `extract_links` stays for index consumers.
pub fn extract_parsed_links(page: &ParsedPage) -> Vec<ParsedLink> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for slug in page.string_list("sources") {
        let raw = slug.to_string();
        if seen.insert(raw.clone()) {
            result.push(ParsedLink::parse(&raw));
        }
    }
    for slug in page.string_list("concepts") {
        let raw = slug.to_string();
        if seen.insert(raw.clone()) {
            result.push(ParsedLink::parse(&raw));
        }
    }
    extract_parsed_wikilinks(&page.body, &mut seen, &mut result);

    result
}

fn extract_parsed_wikilinks(text: &str, seen: &mut HashSet<String>, result: &mut Vec<ParsedLink>) {
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let raw = after[..end].trim().to_string();
            if !raw.is_empty() && seen.insert(raw.clone()) {
                result.push(ParsedLink::parse(&raw));
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    extract_commonmark_links(text, seen, result, None);
}

/// Normalize a CommonMark link destination against the source page's directory.
///
/// `source_dir` is the pre-computed containing directory of the source page:
/// - flat page `technology/concurrency.md`       → `source_dir = "technology"`
/// - bundle page `technology/concurrency/index.md` → `source_dir = "technology/concurrency"`
///
/// Normalizations applied:
/// 1. Strip `.md` suffix
/// 2. Resolve `./` and `../` prefixes against `source_dir`
/// Absolute destinations (no `./` or `../`) are returned unchanged (minus `.md`).
fn normalize_commonmark_dest(dest: &str, source_dir: &str) -> String {
    let dest = dest.strip_suffix(".md").unwrap_or(dest);

    if !dest.starts_with("./") && !dest.starts_with("../") && dest != ".." {
        return dest.to_string();
    }

    let mut parts: Vec<&str> = source_dir.split('/').filter(|s| !s.is_empty()).collect();
    let mut rest = dest;

    loop {
        if let Some(r) = rest.strip_prefix("./") {
            rest = r;
        } else if let Some(r) = rest.strip_prefix("../") {
            parts.pop();
            rest = r;
        } else if rest == ".." {
            parts.pop();
            rest = "";
            break;
        } else {
            break;
        }
    }

    if rest.is_empty() {
        parts.join("/")
    } else {
        let prefix = parts.join("/");
        if prefix.is_empty() {
            rest.to_string()
        } else {
            format!("{}/{}", prefix, rest)
        }
    }
}

/// Extract CommonMark inline link destinations `[text](destination)` from body text.
/// Filters out external URLs, anchors, and image links. Strips `#anchor` suffixes.
fn extract_commonmark_links(text: &str, seen: &mut HashSet<String>, result: &mut Vec<ParsedLink>, source_dir: Option<&str>) {
    let mut rest = text;
    while let Some(bracket) = rest.find("](") {
        let before = &rest[..bracket];
        if let Some(open) = before.rfind('[') {
            // Skip image links — `![alt](`
            let is_image = open > 0 && before.as_bytes()[open - 1] == b'!';
            let after_paren = &rest[bracket + 2..];
            if let Some(close) = after_paren.find(')') {
                let dest_raw = after_paren[..close].trim();
                // Strip #anchor suffix
                let dest = dest_raw
                    .find('#')
                    .map(|i| dest_raw[..i].trim())
                    .unwrap_or(dest_raw);
                if !is_image
                    && !dest.is_empty()
                    && !dest.starts_with("http://")
                    && !dest.starts_with("https://")
                    && !dest.starts_with("mailto:")
                    && !dest.starts_with('#')
                {
                    let raw = match source_dir {
                        Some(dir) => normalize_commonmark_dest(dest, dir),
                        None => dest.to_string(),
                    };
                    if seen.insert(raw.clone()) {
                        result.push(ParsedLink::parse(&raw));
                    }
                }
                rest = &after_paren[close + 1..];
                continue;
            }
        }
        rest = &rest[bracket + 2..];
    }
}

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
    extract_wikilinks(&page.body, &mut seen, &mut result, None);

    result
}

/// Extract `[[slug]]` patterns and CommonMark `[text](destination)` links from body text.
pub fn extract_wikilinks(text: &str, seen: &mut HashSet<String>, result: &mut Vec<String>, source_dir: Option<&str>) {
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
    // Also extract CommonMark inline links, reusing ParsedLink for filtering.
    let mut parsed: Vec<ParsedLink> = Vec::new();
    extract_commonmark_links(text, seen, &mut parsed, source_dir);
    for link in parsed {
        result.push(link.as_raw().to_string());
    }
}

/// Extract only body `[[wikilinks]]` from raw text (no frontmatter parsing).
pub fn extract_body_wikilinks(text: &str, source_dir: Option<&str>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    extract_wikilinks(text, &mut seen, &mut result, source_dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── bundle layout (source_dir == slug) ───────────────────────────────────────

    #[test]
    fn bundle_dotslash_resolves_into_slug_dir() {
        // technology/concurrency/index.md → source_dir = "technology/concurrency"
        // [glossary](./glossary.md) → technology/concurrency/glossary
        let links = extract_body_wikilinks("[glossary](./glossary.md)", Some("technology/concurrency"));
        assert!(
            links.iter().any(|l| l == "technology/concurrency/glossary"),
            "got: {:?}", links
        );
    }

    #[test]
    fn bundle_dotdot_resolves_to_sibling_dir() {
        // technology/concurrency/index.md → source_dir = "technology/concurrency"
        // [patterns](../concurrency/patterns.md) → technology/concurrency/patterns
        let links = extract_body_wikilinks("[patterns](../concurrency/patterns.md)", Some("technology/concurrency"));
        assert!(
            links.iter().any(|l| l == "technology/concurrency/patterns"),
            "got: {:?}", links
        );
    }

    #[test]
    fn bundle_dotdot_no_md_extension() {
        // technology/concurrency/index.md → source_dir = "technology/concurrency"
        // [ractor](../ractor) → technology/ractor
        let links = extract_body_wikilinks("[ractor](../ractor)", Some("technology/concurrency"));
        assert!(
            links.iter().any(|l| l == "technology/ractor"),
            "got: {:?}", links
        );
    }

    // ── flat layout (source_dir == parent of slug) ────────────────────────────

    #[test]
    fn flat_dotslash_resolves_into_parent_dir() {
        // technology/concurrency.md → source_dir = "technology"
        // [glossary](./glossary.md) → technology/glossary
        let links = extract_body_wikilinks("[glossary](./glossary.md)", Some("technology"));
        assert!(
            links.iter().any(|l| l == "technology/glossary"),
            "got: {:?}", links
        );
    }

    #[test]
    fn flat_dotdot_resolves_above_parent_dir() {
        // technology/concurrency.md → source_dir = "technology"
        // [ractor](../ractor) → ractor  (one level up from "technology" is root)
        let links = extract_body_wikilinks("[ractor](../ractor)", Some("technology"));
        assert!(
            links.iter().any(|l| l == "ractor"),
            "got: {:?}", links
        );
    }

    // ── layout-independent cases ──────────────────────────────────────────────

    #[test]
    fn wikilink_absolute_slug_unchanged() {
        let links = extract_body_wikilinks("[[technology/ractor]]", Some("technology/concurrency"));
        assert!(
            links.iter().any(|l| l == "technology/ractor"),
            "got: {:?}", links
        );
    }

    #[test]
    fn external_https_link_excluded() {
        let links = extract_body_wikilinks("[ext](https://example.com)", Some("technology/concurrency"));
        assert!(
            !links.iter().any(|l| l.contains("example.com")),
            "external link should be filtered out, got: {:?}", links
        );
    }

    #[test]
    fn root_level_flat_page_empty_source_dir() {
        // top-level flat page: slug = "glossary", source_dir = "" (rsplit_once returns None → unwrap_or_default)
        // [other](./other.md) → "other" (not "/other", not "other.md")
        let links = extract_body_wikilinks("[other](./other.md)", Some(""));
        assert!(
            links.iter().any(|l| l == "other"),
            "got: {:?}", links
        );
    }

    #[test]
    fn no_source_dir_leaves_relative_dest_unchanged() {
        // extract_links / extract_parsed_links pass None — behavior must be unchanged.
        let links = extract_body_wikilinks("[glossary](./glossary.md)", None);
        assert!(
            links.iter().any(|l| l == "./glossary.md"),
            "without source_dir, raw dest should be preserved, got: {:?}", links
        );
    }
}
