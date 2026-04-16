use llm_wiki::links::extract_links;

#[test]
fn extract_links_returns_slugs_from_sources_frontmatter() {
    let content = "\
---
title: \"Test\"
summary: \"s\"
status: active
last_updated: \"2025-01-01\"
type: concept
sources:
  - sources/paper-a
  - sources/paper-b
---

Body text.
";
    let links = extract_links(content);
    assert!(links.contains(&"sources/paper-a".to_string()));
    assert!(links.contains(&"sources/paper-b".to_string()));
}

#[test]
fn extract_links_returns_slugs_from_concepts_frontmatter() {
    let content = "\
---
title: \"Test\"
summary: \"s\"
status: active
last_updated: \"2025-01-01\"
type: concept
concepts:
  - concepts/scaling-laws
  - concepts/moe
---

Body text.
";
    let links = extract_links(content);
    assert!(links.contains(&"concepts/scaling-laws".to_string()));
    assert!(links.contains(&"concepts/moe".to_string()));
}

#[test]
fn extract_links_returns_slugs_from_body_wikilinks() {
    let content = "\
---
title: \"Test\"
summary: \"s\"
status: active
last_updated: \"2025-01-01\"
type: concept
---

See [[concepts/attention]] and [[sources/transformer-2017]] for details.
";
    let links = extract_links(content);
    assert!(links.contains(&"concepts/attention".to_string()));
    assert!(links.contains(&"sources/transformer-2017".to_string()));
}

#[test]
fn extract_links_deduplicates_repeated_slugs() {
    let content = "\
---
title: \"Test\"
summary: \"s\"
status: active
last_updated: \"2025-01-01\"
type: concept
sources:
  - sources/paper-a
concepts:
  - sources/paper-a
---

Also see [[sources/paper-a]] again.
";
    let links = extract_links(content);
    let count = links.iter().filter(|l| *l == "sources/paper-a").count();
    assert_eq!(count, 1, "should deduplicate: got {links:?}");
}

#[test]
fn extract_links_returns_empty_vec_for_page_with_no_links() {
    let content = "\
---
title: \"Test\"
summary: \"s\"
status: active
last_updated: \"2025-01-01\"
type: concept
---

No links here.
";
    let links = extract_links(content);
    assert!(links.is_empty());
}
