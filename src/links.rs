use std::collections::HashSet;

use crate::frontmatter::parse_frontmatter;

/// Extract all linked slugs from a page's content: frontmatter `sources`, `concepts`, and body `[[wikilinks]]`.
pub fn extract_links(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |s: String| {
        if seen.insert(s.clone()) {
            result.push(s);
        }
    };

    if let Ok((fm, body)) = parse_frontmatter(content) {
        for s in fm.sources {
            push(s);
        }
        for c in fm.concepts {
            push(c);
        }
        extract_wikilinks(&body, &mut seen, &mut result);
    } else {
        extract_wikilinks(content, &mut seen, &mut result);
    }

    result
}

fn extract_wikilinks(text: &str, seen: &mut HashSet<String>, result: &mut Vec<String>) {
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
