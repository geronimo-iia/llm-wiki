use llm_wiki::frontmatter;
use llm_wiki::links::{extract_body_wikilinks, extract_links};

#[test]
fn extract_links_from_sources() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\nsources:\n  - sources/paper-a\n  - sources/paper-b\n---\n\nBody.\n",
    );
    let links = extract_links(&page);
    assert!(links.contains(&"sources/paper-a".to_string()));
    assert!(links.contains(&"sources/paper-b".to_string()));
}

#[test]
fn extract_links_from_concepts() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\nconcepts:\n  - concepts/scaling-laws\n  - concepts/moe\n---\n\nBody.\n",
    );
    let links = extract_links(&page);
    assert!(links.contains(&"concepts/scaling-laws".to_string()));
    assert!(links.contains(&"concepts/moe".to_string()));
}

#[test]
fn extract_links_from_body_wikilinks() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\n---\n\nSee [[concepts/attention]] and [[sources/transformer-2017]].\n",
    );
    let links = extract_links(&page);
    assert!(links.contains(&"concepts/attention".to_string()));
    assert!(links.contains(&"sources/transformer-2017".to_string()));
}

#[test]
fn extract_links_deduplicates() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\nsources:\n  - sources/paper-a\nconcepts:\n  - sources/paper-a\n---\n\nAlso [[sources/paper-a]].\n",
    );
    let links = extract_links(&page);
    let count = links.iter().filter(|l| *l == "sources/paper-a").count();
    assert_eq!(count, 1);
}

#[test]
fn extract_links_empty_when_no_links() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\n---\n\nNo links here.\n",
    );
    let links = extract_links(&page);
    assert!(links.is_empty());
}

#[test]
fn extract_links_no_frontmatter() {
    let page = frontmatter::parse("No frontmatter, just [[concepts/moe]] in body.\n");
    let links = extract_links(&page);
    assert!(links.contains(&"concepts/moe".to_string()));
}

#[test]
fn extract_body_wikilinks_standalone() {
    let links = extract_body_wikilinks("See [[concepts/moe]] and [[sources/paper]].");
    assert_eq!(links, vec!["concepts/moe", "sources/paper"]);
}

#[test]
fn extract_body_wikilinks_trims_whitespace() {
    let links = extract_body_wikilinks("See [[ concepts/moe ]].");
    assert_eq!(links, vec!["concepts/moe"]);
}

#[test]
fn extract_body_wikilinks_ignores_empty() {
    let links = extract_body_wikilinks("See [[]] and [[ ]].");
    assert!(links.is_empty());
}

#[test]
fn extract_body_wikilinks_unclosed_bracket() {
    let links = extract_body_wikilinks("See [[concepts/moe and nothing else.");
    assert!(links.is_empty());
}
