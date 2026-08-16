use llm_wiki::frontmatter;
use llm_wiki::links::{ParsedLink, extract_body_wikilinks, extract_links, extract_parsed_links};

#[test]
fn extract_links_from_sources() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\nsources:\n  - sources/paper-a\n  - sources/paper-b\n---\n\nBody.\n",
        None,
    );
    let links = extract_links(&page);
    assert!(links.contains(&"sources/paper-a".to_string()));
    assert!(links.contains(&"sources/paper-b".to_string()));
}

#[test]
fn extract_links_from_concepts() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\nconcepts:\n  - concepts/scaling-laws\n  - concepts/moe\n---\n\nBody.\n",
        None,
    );
    let links = extract_links(&page);
    assert!(links.contains(&"concepts/scaling-laws".to_string()));
    assert!(links.contains(&"concepts/moe".to_string()));
}

#[test]
fn extract_links_from_body_wikilinks() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\n---\n\nSee [[concepts/attention]] and [[sources/transformer-2017]].\n",
        None,
    );
    let links = extract_links(&page);
    assert!(links.contains(&"concepts/attention".to_string()));
    assert!(links.contains(&"sources/transformer-2017".to_string()));
}

#[test]
fn extract_links_deduplicates() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\nsources:\n  - sources/paper-a\nconcepts:\n  - sources/paper-a\n---\n\nAlso [[sources/paper-a]].\n",
        None,
    );
    let links = extract_links(&page);
    let count = links.iter().filter(|l| *l == "sources/paper-a").count();
    assert_eq!(count, 1);
}

#[test]
fn extract_links_empty_when_no_links() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\n---\n\nNo links here.\n",
        None,
    );
    let links = extract_links(&page);
    assert!(links.is_empty());
}

#[test]
fn extract_links_no_frontmatter() {
    let page = frontmatter::parse("No frontmatter, just [[concepts/moe]] in body.\n", None);
    let links = extract_links(&page);
    assert!(links.contains(&"concepts/moe".to_string()));
}

#[test]
fn extract_body_wikilinks_standalone() {
    let links = extract_body_wikilinks("See [[concepts/moe]] and [[sources/paper]].", None);
    assert_eq!(links, vec!["concepts/moe", "sources/paper"]);
}

#[test]
fn extract_body_wikilinks_trims_whitespace() {
    let links = extract_body_wikilinks("See [[ concepts/moe ]].", None);
    assert_eq!(links, vec!["concepts/moe"]);
}

#[test]
fn extract_body_wikilinks_ignores_empty() {
    let links = extract_body_wikilinks("See [[]] and [[ ]].", None);
    assert!(links.is_empty());
}

#[test]
fn extract_body_wikilinks_unclosed_bracket() {
    let links = extract_body_wikilinks("See [[concepts/moe and nothing else.", None);
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
        None,
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
    let links = extract_body_wikilinks("[Foo](concepts/foo)", None);
    assert_eq!(links, vec!["concepts/foo"]);
}

#[test]
fn commonmark_cross_wiki_link_in_body() {
    let page = frontmatter::parse(
        "---\ntitle: \"Test\"\ntype: concept\n---\n\nSee [MoE](wiki://research/concepts/moe).\n",
        None,
    );
    let links = extract_parsed_links(&page);
    assert!(links.contains(&ParsedLink::CrossWiki {
        wiki: "research".to_string(),
        slug: "concepts/moe".to_string(),
    }));
}

#[test]
fn commonmark_external_url_filtered() {
    let links = extract_body_wikilinks("[Google](https://google.com)", None);
    assert!(links.is_empty());
}

#[test]
fn commonmark_anchor_filtered() {
    let links = extract_body_wikilinks("[Top](#top)", None);
    assert!(links.is_empty());
}

#[test]
fn commonmark_mixed_wikilink_and_commonmark() {
    let links = extract_body_wikilinks("See [[concepts/foo]] and [bar](concepts/bar).", None);
    assert_eq!(links, vec!["concepts/foo", "concepts/bar"]);
}

#[test]
fn commonmark_deduplication_across_syntaxes() {
    let links = extract_body_wikilinks("[[concepts/foo]] and [also](concepts/foo)", None);
    assert_eq!(links, vec!["concepts/foo"]);
}

#[test]
fn commonmark_image_link_filtered() {
    let links = extract_body_wikilinks("![alt](image.png)", None);
    assert!(links.is_empty());
}

// ── Code block / inline code exclusion (issue #127) ──────────────────────────

#[test]
fn wikilinks_not_extracted_from_fenced_code_block() {
    // TOML [[section]] headers inside fenced blocks must not become wikilinks
    let body = "See [[real-link]].\n\n```toml\n[[bench]]\nname = \"my_bench\"\n\n[[pre-release-hooks]]\ncommand = \"cargo\"\n```\n\nAlso [[another-link]].\n";
    let links = extract_body_wikilinks(body, None);
    assert!(links.contains(&"real-link".to_string()), "real-link missing: {links:?}");
    assert!(links.contains(&"another-link".to_string()), "another-link missing: {links:?}");
    assert!(!links.contains(&"bench".to_string()), "bench must NOT be extracted: {links:?}");
    assert!(!links.contains(&"pre-release-hooks".to_string()), "pre-release-hooks must NOT be extracted: {links:?}");
}

#[test]
fn wikilinks_not_extracted_from_inline_code() {
    let body = "Use `[[not-a-link]]` to configure, but see [[real-link]] for details.";
    let links = extract_body_wikilinks(body, None);
    assert!(links.contains(&"real-link".to_string()), "real-link missing: {links:?}");
    assert!(!links.contains(&"not-a-link".to_string()), "not-a-link must NOT be extracted: {links:?}");
}
