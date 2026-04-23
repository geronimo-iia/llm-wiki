use std::fs;

use llm_wiki::frontmatter;
use llm_wiki::markdown::*;
use llm_wiki::slug::Slug;

fn setup_wiki(dir: &std::path::Path) -> std::path::PathBuf {
    let wiki_root = dir.join("wiki");
    fs::create_dir_all(&wiki_root).unwrap();
    wiki_root
}

fn write_file(wiki_root: &std::path::Path, rel_path: &str, content: &str) {
    let path = wiki_root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

const SAMPLE: &str =
    "---\ntitle: \"Test\"\ntype: concept\nstatus: active\n---\n\n## Overview\n\nHello world.\n";

fn slug(s: &str) -> Slug {
    Slug::try_from(s).unwrap()
}

// ── read_page ─────────────────────────────────────────────────────────────────

#[test]
fn read_page_returns_full_content() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo.md", SAMPLE);

    let content = read_page(&slug("concepts/foo"), &wiki, false).unwrap();
    assert!(content.starts_with("---"));
    assert!(content.contains("title: \"Test\""));
    assert!(content.contains("## Overview"));
}

#[test]
fn read_page_strips_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo.md", SAMPLE);

    let content = read_page(&slug("concepts/foo"), &wiki, true).unwrap();
    assert!(!content.contains("---"));
    assert!(!content.contains("title:"));
    assert!(content.contains("## Overview"));
}

#[test]
fn read_page_supersession_notice() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(
        &wiki,
        "concepts/old.md",
        "---\ntitle: \"Old\"\ntype: concept\nsuperseded_by: concepts/new\n---\n\nOld content.\n",
    );

    let content = read_page(&slug("concepts/old"), &wiki, false).unwrap();
    assert!(content.contains("Superseded"));
    assert!(content.contains("concepts/new"));
}

#[test]
fn read_page_no_supersession_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/current.md", SAMPLE);

    let content = read_page(&slug("concepts/current"), &wiki, false).unwrap();
    assert!(!content.contains("Superseded"));
}

// ── write_page ────────────────────────────────────────────────────────────────

#[test]
fn write_page_creates_new_file() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    let path = write_page("concepts/new", "# New\n", &wiki).unwrap();
    assert!(path.exists());
    assert_eq!(fs::read_to_string(&path).unwrap(), "# New\n");
}

#[test]
fn write_page_overwrites_existing() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo.md", SAMPLE);

    write_page("concepts/foo", "# Updated\n", &wiki).unwrap();
    let content = fs::read_to_string(wiki.join("concepts/foo.md")).unwrap();
    assert_eq!(content, "# Updated\n");
}

#[test]
fn write_page_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    write_page("deep/nested/page", "content\n", &wiki).unwrap();
    assert!(wiki.join("deep/nested/page.md").exists());
}

// ── list_assets ───────────────────────────────────────────────────────────────

#[test]
fn list_assets_empty_for_flat_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo.md", SAMPLE);

    let assets = list_assets(&slug("concepts/foo"), &wiki).unwrap();
    assert!(assets.is_empty());
}

#[test]
fn list_assets_returns_bundle_files() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo/index.md", SAMPLE);
    fs::write(wiki.join("concepts/foo/diagram.png"), b"fake").unwrap();
    fs::write(wiki.join("concepts/foo/config.yaml"), b"key: val").unwrap();

    let assets = list_assets(&slug("concepts/foo"), &wiki).unwrap();
    assert_eq!(assets.len(), 2);
    assert!(assets.contains(&"wiki://concepts/foo/config.yaml".to_string()));
    assert!(assets.contains(&"wiki://concepts/foo/diagram.png".to_string()));
}

// ── read_asset ────────────────────────────────────────────────────────────────

#[test]
fn read_asset_returns_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo/index.md", SAMPLE);
    fs::write(wiki.join("concepts/foo/data.bin"), b"\x00\x01\x02").unwrap();

    let bytes = read_asset(&slug("concepts/foo"), "data.bin", &wiki).unwrap();
    assert_eq!(bytes, b"\x00\x01\x02");
}

#[test]
fn read_asset_missing_errors() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo/index.md", SAMPLE);

    assert!(read_asset(&slug("concepts/foo"), "nope.png", &wiki).is_err());
}

// ── create_page ───────────────────────────────────────────────────────────────

#[test]
fn create_page_flat() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    let path = create_page(&slug("concepts/bar"), false, &wiki, None, None, None).unwrap();
    assert_eq!(path, wiki.join("concepts/bar.md"));
    assert!(path.is_file());

    let page = frontmatter::parse(&fs::read_to_string(&path).unwrap());
    assert_eq!(page.title(), Some("Bar"));
    assert_eq!(page.page_type(), Some("page"));
    assert_eq!(page.status(), Some("draft"));
}

#[test]
fn create_page_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    let path = create_page(&slug("concepts/bar"), true, &wiki, None, None, None).unwrap();
    assert_eq!(path, wiki.join("concepts/bar/index.md"));
    assert!(path.is_file());
}

#[test]
fn create_page_with_name_override() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    let path = create_page(
        &slug("concepts/bar"),
        false,
        &wiki,
        Some("Custom Title"),
        None,
        None,
    )
    .unwrap();
    let page = frontmatter::parse(&fs::read_to_string(&path).unwrap());
    assert_eq!(page.title(), Some("Custom Title"));
}

#[test]
fn create_page_with_type_override() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    let path = create_page(
        &slug("concepts/bar"),
        false,
        &wiki,
        None,
        Some("paper"),
        None,
    )
    .unwrap();
    let page = frontmatter::parse(&fs::read_to_string(&path).unwrap());
    assert_eq!(page.page_type(), Some("paper"));
}

#[test]
fn create_page_auto_creates_parent_sections() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    create_page(&slug("a/b/c"), false, &wiki, None, None, None).unwrap();

    let a_index = wiki.join("a/index.md");
    assert!(a_index.is_file());
    let page = frontmatter::parse(&fs::read_to_string(&a_index).unwrap());
    assert_eq!(page.page_type(), Some("section"));
    assert_eq!(page.title(), Some("A"));

    let ab_index = wiki.join("a/b/index.md");
    assert!(ab_index.is_file());
    let page = frontmatter::parse(&fs::read_to_string(&ab_index).unwrap());
    assert_eq!(page.page_type(), Some("section"));

    assert!(wiki.join("a/b/c.md").is_file());
}

// ── create_section ────────────────────────────────────────────────────────────

#[test]
fn create_section_creates_index_md() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    let path = create_section(&slug("skills"), &wiki, None).unwrap();
    assert_eq!(path, wiki.join("skills/index.md"));
    assert!(path.is_file());

    let page = frontmatter::parse(&fs::read_to_string(&path).unwrap());
    assert_eq!(page.title(), Some("Skills"));
    assert_eq!(page.page_type(), Some("section"));
    assert_eq!(page.status(), Some("draft"));
}

// ── promote_to_bundle ─────────────────────────────────────────────────────────

#[test]
fn promote_to_bundle_moves_flat_to_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo.md", SAMPLE);

    promote_to_bundle(&slug("concepts/foo"), &wiki).unwrap();

    assert!(!wiki.join("concepts/foo.md").exists());
    assert!(wiki.join("concepts/foo/index.md").is_file());
}

#[test]
fn promote_to_bundle_resolves_after() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());
    write_file(&wiki, "concepts/foo.md", SAMPLE);

    promote_to_bundle(&slug("concepts/foo"), &wiki).unwrap();

    let path = slug("concepts/foo").resolve(&wiki).unwrap();
    assert_eq!(path, wiki.join("concepts/foo/index.md"));
}

#[test]
fn promote_to_bundle_missing_errors() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = setup_wiki(dir.path());

    assert!(promote_to_bundle(&slug("concepts/nope"), &wiki).is_err());
}
