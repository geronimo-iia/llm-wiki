use llm_wiki::frontmatter;
use llm_wiki::links::{ParsedLink, extract_body_wikilinks, extract_links, extract_parsed_links};

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
    let page = frontmatter::parse("---\ntitle: \"Test\"\ntype: concept\n---\n\nNo links here.\n");
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

// ── ParsedLink ────────────────────────────────────────────────────────────────

#[test]
fn parsed_link_local() {
    assert_eq!(
        ParsedLink::parse("concepts/moe"),
        ParsedLink::Local("concepts/moe".to_string())
    );
}

#[test]
fn parsed_link_cross_wiki() {
    assert_eq!(
        ParsedLink::parse("wiki://other/concepts/foo"),
        ParsedLink::CrossWiki {
            wiki: "other".to_string(),
            slug: "concepts/foo".to_string(),
        }
    );
}

#[test]
fn parsed_link_cross_wiki_no_slash_is_local() {
    // "wiki://nopath" has no slash after the wiki name — treated as local
    assert_eq!(
        ParsedLink::parse("wiki://nopath"),
        ParsedLink::Local("wiki://nopath".to_string())
    );
}

#[test]
fn extract_parsed_links_returns_cross_wiki_variant() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\nsources:\n  - wiki://other/concepts/foo\n  - concepts/local\n---\n\nBody with [[wiki://third/bar]].\n",
    );
    let links = extract_parsed_links(&page);
    assert!(links.contains(&ParsedLink::CrossWiki {
        wiki: "other".to_string(),
        slug: "concepts/foo".to_string(),
    }));
    assert!(links.contains(&ParsedLink::Local("concepts/local".to_string())));
    assert!(links.contains(&ParsedLink::CrossWiki {
        wiki: "third".to_string(),
        slug: "bar".to_string(),
    }));
}

// ── CommonMark inline links ───────────────────────────────────────────────────

#[test]
fn commonmark_basic_local_link() {
    let links = extract_body_wikilinks("[Foo](concepts/foo)");
    assert_eq!(links, vec!["concepts/foo"]);
}

#[test]
fn commonmark_cross_wiki_link_in_body() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\n---\n\nSee [MoE](wiki://research/concepts/moe).\n",
    );
    let links = extract_parsed_links(&page);
    assert!(links.contains(&ParsedLink::CrossWiki {
        wiki: "research".to_string(),
        slug: "concepts/moe".to_string(),
    }));
}

#[test]
fn commonmark_external_url_filtered() {
    let links = extract_body_wikilinks("[Google](https://google.com)");
    assert!(links.is_empty());
}

#[test]
fn commonmark_anchor_filtered() {
    let links = extract_body_wikilinks("[Top](#top)");
    assert!(links.is_empty());
}

#[test]
fn commonmark_mixed_wikilink_and_commonmark() {
    let links = extract_body_wikilinks("See [[concepts/foo]] and [bar](concepts/bar).");
    assert_eq!(links, vec!["concepts/foo", "concepts/bar"]);
}

#[test]
fn commonmark_deduplication_across_syntaxes() {
    let links = extract_body_wikilinks("[[concepts/foo]] and [also](concepts/foo)");
    assert_eq!(links, vec!["concepts/foo"]);
}

#[test]
fn commonmark_image_link_filtered() {
    let links = extract_body_wikilinks("![alt](image.png)");
    assert!(links.is_empty());
}
