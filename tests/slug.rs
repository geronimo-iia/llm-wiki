use std::path::Path;

use llm_wiki::slug::{resolve_read_target, ReadTarget, Slug, WikiUri};

// ── Slug construction ─────────────────────────────────────────────────────────

#[test]
fn slug_valid() {
    let s = Slug::try_from("concepts/moe").unwrap();
    assert_eq!(s.as_str(), "concepts/moe");
}

#[test]
fn slug_single_segment() {
    let s = Slug::try_from("readme").unwrap();
    assert_eq!(s.as_str(), "readme");
}

#[test]
fn slug_rejects_empty() {
    assert!(Slug::try_from("").is_err());
}

#[test]
fn slug_rejects_leading_slash() {
    assert!(Slug::try_from("/concepts/moe").is_err());
}

#[test]
fn slug_rejects_traversal() {
    assert!(Slug::try_from("concepts/../secrets").is_err());
}

#[test]
fn slug_rejects_extension() {
    assert!(Slug::try_from("concepts/moe.md").is_err());
}

#[test]
fn slug_trims_whitespace() {
    let s = Slug::try_from("  concepts/moe  ").unwrap();
    assert_eq!(s.as_str(), "concepts/moe");
}

// ── Slug::from_path ───────────────────────────────────────────────────────────

#[test]
fn from_path_flat() {
    let root = Path::new("/wiki");
    let path = Path::new("/wiki/concepts/moe.md");
    let s = Slug::from_path(path, root).unwrap();
    assert_eq!(s.as_str(), "concepts/moe");
}

#[test]
fn from_path_bundle() {
    let root = Path::new("/wiki");
    let path = Path::new("/wiki/concepts/moe/index.md");
    let s = Slug::from_path(path, root).unwrap();
    assert_eq!(s.as_str(), "concepts/moe");
}

#[test]
fn from_path_outside_root() {
    let root = Path::new("/wiki");
    let path = Path::new("/other/concepts/moe.md");
    assert!(Slug::from_path(path, root).is_err());
}

// ── Slug::title ───────────────────────────────────────────────────────────────

#[test]
fn title_from_slug() {
    let s = Slug::try_from("concepts/mixture-of-experts").unwrap();
    assert_eq!(s.title(), "Mixture Of Experts");
}

#[test]
fn title_single_segment() {
    let s = Slug::try_from("readme").unwrap();
    assert_eq!(s.title(), "Readme");
}

// ── Slug::resolve ─────────────────────────────────────────────────────────────

#[test]
fn resolve_flat() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = dir.path();
    std::fs::create_dir_all(wiki.join("concepts")).unwrap();
    std::fs::write(wiki.join("concepts/moe.md"), "# MoE").unwrap();

    let s = Slug::try_from("concepts/moe").unwrap();
    let path = s.resolve(wiki).unwrap();
    assert_eq!(path, wiki.join("concepts/moe.md"));
}

#[test]
fn resolve_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = dir.path();
    std::fs::create_dir_all(wiki.join("concepts/moe")).unwrap();
    std::fs::write(wiki.join("concepts/moe/index.md"), "# MoE").unwrap();

    let s = Slug::try_from("concepts/moe").unwrap();
    let path = s.resolve(wiki).unwrap();
    assert_eq!(path, wiki.join("concepts/moe/index.md"));
}

#[test]
fn resolve_flat_wins_over_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = dir.path();
    std::fs::create_dir_all(wiki.join("concepts/moe")).unwrap();
    std::fs::write(wiki.join("concepts/moe.md"), "flat").unwrap();
    std::fs::write(wiki.join("concepts/moe/index.md"), "bundle").unwrap();

    let s = Slug::try_from("concepts/moe").unwrap();
    let path = s.resolve(wiki).unwrap();
    assert_eq!(path, wiki.join("concepts/moe.md"));
}

#[test]
fn resolve_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let s = Slug::try_from("concepts/moe").unwrap();
    assert!(s.resolve(dir.path()).is_err());
}

// ── WikiUri::parse ────────────────────────────────────────────────────────────

#[test]
fn parse_bare_slug() {
    let uri = WikiUri::parse("concepts/moe").unwrap();
    assert_eq!(uri.wiki, None);
    assert_eq!(uri.slug.as_str(), "concepts/moe");
}

#[test]
fn parse_full_uri() {
    let uri = WikiUri::parse("wiki://research/concepts/moe").unwrap();
    assert_eq!(uri.wiki, Some("research".into()));
    assert_eq!(uri.slug.as_str(), "concepts/moe");
}

#[test]
fn parse_uri_ambiguous() {
    // wiki://concepts/moe — "concepts" stored as candidate wiki name
    let uri = WikiUri::parse("wiki://concepts/moe").unwrap();
    assert_eq!(uri.wiki, Some("concepts".into()));
    assert_eq!(uri.slug.as_str(), "moe");
}

#[test]
fn parse_uri_single_segment() {
    let uri = WikiUri::parse("wiki://readme").unwrap();
    assert_eq!(uri.wiki, None);
    assert_eq!(uri.slug.as_str(), "readme");
}

#[test]
fn parse_empty_uri_fails() {
    assert!(WikiUri::parse("wiki://").is_err());
}

// ── resolve_read_target ───────────────────────────────────────────────────────

#[test]
fn read_target_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = dir.path();
    std::fs::create_dir_all(wiki.join("concepts")).unwrap();
    std::fs::write(wiki.join("concepts/moe.md"), "# MoE").unwrap();

    match resolve_read_target("concepts/moe", wiki).unwrap() {
        ReadTarget::Page(p) => assert_eq!(p, wiki.join("concepts/moe.md")),
        ReadTarget::Asset(..) => panic!("expected Page"),
    }
}

#[test]
fn read_target_asset() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = dir.path();
    std::fs::create_dir_all(wiki.join("concepts/moe")).unwrap();
    std::fs::write(wiki.join("concepts/moe/index.md"), "# MoE").unwrap();
    std::fs::write(wiki.join("concepts/moe/diagram.png"), "PNG").unwrap();

    match resolve_read_target("concepts/moe/diagram.png", wiki).unwrap() {
        ReadTarget::Asset(parent, filename) => {
            assert_eq!(parent, "concepts/moe");
            assert_eq!(filename, "diagram.png");
        }
        ReadTarget::Page(_) => panic!("expected Asset"),
    }
}

#[test]
fn read_target_not_found() {
    let dir = tempfile::tempdir().unwrap();
    assert!(resolve_read_target("concepts/moe", dir.path()).is_err());
}

// ── Display / AsRef ───────────────────────────────────────────────────────────

#[test]
fn slug_display() {
    let s = Slug::try_from("concepts/moe").unwrap();
    assert_eq!(format!("{s}"), "concepts/moe");
}

#[test]
fn slug_as_ref() {
    let s = Slug::try_from("concepts/moe").unwrap();
    let r: &str = s.as_ref();
    assert_eq!(r, "concepts/moe");
}

// ── WikiUri::resolve ────────────────────────────────────────────────────

use llm_wiki::config::{GlobalConfig, GlobalSection, WikiEntry};

fn make_global(wikis: Vec<WikiEntry>, default: &str) -> GlobalConfig {
    GlobalConfig {
        global: GlobalSection {
            default_wiki: default.into(),
        },
        wikis,
        ..Default::default()
    }
}

fn make_entry(name: &str, path: &str) -> WikiEntry {
    WikiEntry {
        name: name.into(),
        path: path.into(),
        description: None,
        remote: None,
    }
}

#[test]
fn resolve_full_uri() {
    let global = make_global(vec![make_entry("research", "/tmp/research")], "research");
    let (entry, slug) = WikiUri::resolve("wiki://research/concepts/moe", None, &global).unwrap();
    assert_eq!(entry.name, "research");
    assert_eq!(slug.as_str(), "concepts/moe");
}

#[test]
fn resolve_uri_falls_back_to_default() {
    let global = make_global(vec![make_entry("research", "/tmp/research")], "research");
    // "concepts" is not a wiki name → treat as slug segment under default wiki
    let (entry, slug) = WikiUri::resolve("wiki://concepts/moe", None, &global).unwrap();
    assert_eq!(entry.name, "research");
    assert_eq!(slug.as_str(), "concepts/moe");
}

#[test]
fn resolve_bare_slug_uses_default() {
    let global = make_global(vec![make_entry("research", "/tmp/research")], "research");
    let (entry, slug) = WikiUri::resolve("concepts/moe", None, &global).unwrap();
    assert_eq!(entry.name, "research");
    assert_eq!(slug.as_str(), "concepts/moe");
}

#[test]
fn resolve_bare_slug_uses_wiki_flag() {
    let global = make_global(
        vec![
            make_entry("research", "/tmp/research"),
            make_entry("work", "/tmp/work"),
        ],
        "research",
    );
    let (entry, slug) = WikiUri::resolve("concepts/moe", Some("work"), &global).unwrap();
    assert_eq!(entry.name, "work");
    assert_eq!(slug.as_str(), "concepts/moe");
}

#[test]
fn resolve_uri_ignores_wiki_flag() {
    let global = make_global(
        vec![
            make_entry("research", "/tmp/research"),
            make_entry("work", "/tmp/work"),
        ],
        "research",
    );
    // wiki:// URI specifies "research" explicitly — wiki_flag "work" is ignored
    let (entry, slug) = WikiUri::resolve("wiki://research/concepts/moe", Some("work"), &global).unwrap();
    assert_eq!(entry.name, "research");
    assert_eq!(slug.as_str(), "concepts/moe");
}

#[test]
fn resolve_unknown_wiki_errors() {
    let global = make_global(vec![], "");
    assert!(WikiUri::resolve("concepts/moe", None, &global).is_err());
}
