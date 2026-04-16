use std::fs;

use llm_wiki::frontmatter::parse_frontmatter;
use llm_wiki::markdown::*;

fn setup_wiki(dir: &std::path::Path) -> std::path::PathBuf {
    let wiki_root = dir.join("wiki");
    fs::create_dir_all(&wiki_root).unwrap();
    wiki_root
}

fn write_page(wiki_root: &std::path::Path, rel_path: &str, content: &str) {
    let path = wiki_root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

const SAMPLE_PAGE: &str = "---\ntitle: \"Test\"\nsummary: \"A test\"\nstatus: active\nlast_updated: \"2025-07-15\"\ntype: concept\n---\n\n## Overview\n\nHello world.\n";

#[test]
fn slug_for_flat_file_returns_path_without_extension() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    let path = wiki_root.join("concepts/scaling-laws.md");
    assert_eq!(slug_for(&path, &wiki_root), "concepts/scaling-laws");
}

#[test]
fn slug_for_bundle_index_returns_parent_directory_path() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    let path = wiki_root.join("concepts/mixture-of-experts/index.md");
    assert_eq!(slug_for(&path, &wiki_root), "concepts/mixture-of-experts");
}

#[test]
fn resolve_slug_finds_flat_md_file() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo.md", SAMPLE_PAGE);

    let resolved = resolve_slug("concepts/foo", &wiki_root).unwrap();
    assert_eq!(resolved, wiki_root.join("concepts/foo.md"));
}

#[test]
fn resolve_slug_finds_bundle_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo/index.md", SAMPLE_PAGE);

    let resolved = resolve_slug("concepts/foo", &wiki_root).unwrap();
    assert_eq!(resolved, wiki_root.join("concepts/foo/index.md"));
}

#[test]
fn resolve_slug_returns_error_for_missing_slug() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    assert!(resolve_slug("concepts/nonexistent", &wiki_root).is_err());
}

#[test]
fn read_page_returns_full_content_including_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo.md", SAMPLE_PAGE);

    let content = read_page("concepts/foo", &wiki_root, false).unwrap();
    assert!(content.starts_with("---"));
    assert!(content.contains("title: \"Test\""));
    assert!(content.contains("## Overview"));
}

#[test]
fn read_page_with_no_frontmatter_strips_frontmatter_block() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo.md", SAMPLE_PAGE);

    let content = read_page("concepts/foo", &wiki_root, true).unwrap();
    assert!(!content.contains("---"));
    assert!(!content.contains("title:"));
    assert!(content.contains("## Overview"));
}

#[test]
fn list_assets_returns_empty_vec_for_flat_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo.md", SAMPLE_PAGE);

    let assets = list_assets("concepts/foo", &wiki_root).unwrap();
    assert!(assets.is_empty());
}

#[test]
fn list_assets_returns_wiki_uris_for_bundle_assets() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo/index.md", SAMPLE_PAGE);
    fs::write(wiki_root.join("concepts/foo/diagram.png"), b"fake png").unwrap();
    fs::write(wiki_root.join("concepts/foo/config.yaml"), b"key: val").unwrap();

    let assets = list_assets("concepts/foo", &wiki_root).unwrap();
    assert_eq!(assets.len(), 2);
    assert!(assets.contains(&"wiki://concepts/foo/config.yaml".to_string()));
    assert!(assets.contains(&"wiki://concepts/foo/diagram.png".to_string()));
}

#[test]
fn read_asset_returns_raw_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo/index.md", SAMPLE_PAGE);
    fs::write(wiki_root.join("concepts/foo/data.bin"), b"\x00\x01\x02").unwrap();

    let bytes = read_asset("concepts/foo", "data.bin", &wiki_root).unwrap();
    assert_eq!(bytes, b"\x00\x01\x02");
}

#[test]
fn promote_to_bundle_moves_flat_to_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo.md", SAMPLE_PAGE);

    promote_to_bundle("concepts/foo", &wiki_root).unwrap();

    assert!(!wiki_root.join("concepts/foo.md").exists());
    assert!(wiki_root.join("concepts/foo/index.md").is_file());
}

#[test]
fn promote_to_bundle_slug_resolves_after_promotion() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo.md", SAMPLE_PAGE);

    promote_to_bundle("concepts/foo", &wiki_root).unwrap();

    let resolved = resolve_slug("concepts/foo", &wiki_root).unwrap();
    assert_eq!(resolved, wiki_root.join("concepts/foo/index.md"));
}

#[test]
fn create_page_creates_flat_md_with_scaffold_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());

    let path = create_page("concepts/bar", false, &wiki_root).unwrap();
    assert_eq!(path, wiki_root.join("concepts/bar.md"));
    assert!(path.is_file());

    let content = fs::read_to_string(&path).unwrap();
    let (fm, _) = parse_frontmatter(&content).unwrap();
    assert_eq!(fm.title, "Bar");
    assert_eq!(fm.status, "draft");
    assert_eq!(fm.r#type, "page");
}

#[test]
fn create_page_with_bundle_creates_index_md() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());

    let path = create_page("concepts/bar", true, &wiki_root).unwrap();
    assert_eq!(path, wiki_root.join("concepts/bar/index.md"));
    assert!(path.is_file());
}

#[test]
fn create_page_auto_creates_missing_parent_sections() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());

    create_page("a/b/c", false, &wiki_root).unwrap();

    // Parent sections should have index.md with section type
    let a_index = wiki_root.join("a/index.md");
    assert!(a_index.is_file());
    let content = fs::read_to_string(&a_index).unwrap();
    let (fm, _) = parse_frontmatter(&content).unwrap();
    assert_eq!(fm.r#type, "section");
    assert_eq!(fm.title, "A");

    let ab_index = wiki_root.join("a/b/index.md");
    assert!(ab_index.is_file());
    let content = fs::read_to_string(&ab_index).unwrap();
    let (fm, _) = parse_frontmatter(&content).unwrap();
    assert_eq!(fm.r#type, "section");
    assert_eq!(fm.title, "B");

    // The page itself
    assert!(wiki_root.join("a/b/c.md").is_file());
}

#[test]
fn create_section_creates_index_md_with_section_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());

    let path = create_section("skills", &wiki_root).unwrap();
    assert_eq!(path, wiki_root.join("skills/index.md"));
    assert!(path.is_file());

    let content = fs::read_to_string(&path).unwrap();
    let (fm, _) = parse_frontmatter(&content).unwrap();
    assert_eq!(fm.title, "Skills");
    assert_eq!(fm.r#type, "section");
    assert_eq!(fm.status, "draft");
}

// ── resolve_read_target ───────────────────────────────────────────────────────

#[test]
fn resolve_read_target_returns_page_for_flat_slug() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/foo.md", SAMPLE_PAGE);

    match resolve_read_target("concepts/foo", &wiki_root).unwrap() {
        ReadTarget::Page(path) => assert_eq!(path, wiki_root.join("concepts/foo.md")),
        ReadTarget::Asset(_, _) => panic!("expected Page, got Asset"),
    }
}

#[test]
fn resolve_read_target_returns_page_for_bundle_slug() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/moe/index.md", SAMPLE_PAGE);

    match resolve_read_target("concepts/moe", &wiki_root).unwrap() {
        ReadTarget::Page(path) => assert_eq!(path, wiki_root.join("concepts/moe/index.md")),
        ReadTarget::Asset(_, _) => panic!("expected Page, got Asset"),
    }
}

#[test]
fn resolve_read_target_returns_asset_for_bundle_file() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/moe/index.md", SAMPLE_PAGE);
    fs::write(wiki_root.join("concepts/moe/diagram.png"), b"fake png").unwrap();

    match resolve_read_target("concepts/moe/diagram.png", &wiki_root).unwrap() {
        ReadTarget::Asset(parent, filename) => {
            assert_eq!(parent, "concepts/moe");
            assert_eq!(filename, "diagram.png");
        }
        ReadTarget::Page(_) => panic!("expected Asset, got Page"),
    }
}

#[test]
fn resolve_read_target_returns_error_for_missing_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());

    let result = resolve_read_target("concepts/missing", &wiki_root);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("page not found"));
}

#[test]
fn resolve_read_target_returns_error_for_missing_asset() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/moe/index.md", SAMPLE_PAGE);

    let result = resolve_read_target("concepts/moe/missing.png", &wiki_root);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("asset not found"));
}

#[test]
fn resolve_read_target_page_wins_over_asset_for_bundle_slug() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_wiki(dir.path());
    write_page(&wiki_root, "concepts/moe/index.md", SAMPLE_PAGE);
    fs::write(wiki_root.join("concepts/moe/diagram.png"), b"fake").unwrap();

    // "concepts/moe" should resolve as page (step 1), not try asset path
    match resolve_read_target("concepts/moe", &wiki_root).unwrap() {
        ReadTarget::Page(_) => {}
        ReadTarget::Asset(_, _) => panic!("expected Page, got Asset"),
    }
}
