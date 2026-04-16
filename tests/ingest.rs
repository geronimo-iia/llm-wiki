use std::fs;
use std::path::Path;

use llm_wiki::config::{SchemaConfig, ValidationConfig};
use llm_wiki::frontmatter::parse_frontmatter;
use llm_wiki::git;
use llm_wiki::ingest::*;

fn default_schema() -> SchemaConfig {
    SchemaConfig::default()
}

fn default_validation() -> ValidationConfig {
    ValidationConfig::default()
}

fn setup_repo(dir: &Path) -> std::path::PathBuf {
    let wiki_root = dir.join("wiki");
    fs::create_dir_all(&wiki_root).unwrap();
    fs::create_dir_all(dir.join("inbox")).unwrap();
    fs::create_dir_all(dir.join("raw")).unwrap();
    git::init_repo(dir).unwrap();
    // Initial commit so HEAD exists
    fs::write(dir.join("README.md"), "# test\n").unwrap();
    git::commit(dir, "init").unwrap();
    wiki_root
}

fn write_page(wiki_root: &Path, rel_path: &str, content: &str) {
    let path = wiki_root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

const VALID_PAGE: &str = "\
---
title: \"Test Page\"
summary: \"A test\"
status: active
last_updated: \"2025-01-01\"
type: concept
---

## Body
";

#[test]
fn ingest_validates_a_valid_page_and_commits() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let opts = IngestOptions { dry_run: false, auto_commit: true };
    let report = ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();

    assert_eq!(report.pages_validated, 1);
    assert_eq!(report.assets_found, 0);
    assert!(!report.commit.is_empty());
}

#[test]
fn ingest_rejects_page_with_no_title() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/bad.md",
        "---\ntitle: \"\"\nstatus: active\ntype: concept\n---\n\nBody\n",
    );

    let opts = IngestOptions { dry_run: false, ..Default::default() };
    let result = ingest(
        Path::new("concepts/bad.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title"));
}

#[test]
fn ingest_rejects_page_with_invalid_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/bad.md",
        "---\ntitle: [broken yaml {{\n---\n\nBody\n",
    );

    let opts = IngestOptions { dry_run: false, ..Default::default() };
    let result = ingest(
        Path::new("concepts/bad.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    );
    assert!(result.is_err());
}

#[test]
fn ingest_generates_minimal_frontmatter_for_file_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/bare.md",
        "# My Bare Page\n\nJust content.\n",
    );

    let opts = IngestOptions { dry_run: false, ..Default::default() };
    let report = ingest(
        Path::new("concepts/bare.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();
    assert_eq!(report.pages_validated, 1);

    let content = fs::read_to_string(wiki_root.join("concepts/bare.md")).unwrap();
    let (fm, body) = parse_frontmatter(&content).unwrap();
    assert_eq!(fm.title, "My Bare Page");
    assert_eq!(fm.status, "active");
    assert_eq!(fm.r#type, "page");
    assert!(body.contains("# My Bare Page"));
}

#[test]
fn ingest_sets_last_updated_to_today() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let opts = IngestOptions { dry_run: false, ..Default::default() };
    ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();

    let content = fs::read_to_string(wiki_root.join("concepts/foo.md")).unwrap();
    let (fm, _) = parse_frontmatter(&content).unwrap();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    assert_eq!(fm.last_updated, today);
}

#[test]
fn ingest_dry_run_does_not_commit() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let head_before = git::current_head(dir.path()).unwrap();

    let opts = IngestOptions { dry_run: true, ..Default::default() };
    let report = ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();

    assert_eq!(report.pages_validated, 1);
    assert!(report.commit.is_empty());

    let head_after = git::current_head(dir.path()).unwrap();
    assert_eq!(head_before, head_after);
}

#[test]
fn ingest_folder_ingests_all_md_files_recursively() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/a.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/sub/b.md", VALID_PAGE);

    let opts = IngestOptions { dry_run: false, ..Default::default() };
    let report = ingest(
        Path::new("concepts"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();
    assert_eq!(report.pages_validated, 2);
}

#[test]
fn ingest_detects_colocated_assets() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo/index.md", VALID_PAGE);
    fs::write(wiki_root.join("concepts/foo/diagram.png"), b"fake").unwrap();
    fs::write(wiki_root.join("concepts/foo/config.yaml"), b"key: val").unwrap();

    let opts = IngestOptions { dry_run: false, ..Default::default() };
    let report = ingest(
        Path::new("concepts/foo"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();
    assert_eq!(report.pages_validated, 1);
    assert_eq!(report.assets_found, 2);
}

#[test]
fn ingest_report_commit_matches_git_head() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let opts = IngestOptions { dry_run: false, auto_commit: true };
    let report = ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();

    let head = git::current_head(dir.path()).unwrap();
    assert_eq!(report.commit, head);
}

#[test]
fn ingest_rebuilds_index_when_auto_rebuild_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    let index_path = dir.path().join("index-store");

    // Build initial empty index
    llm_wiki::search::rebuild_index(&wiki_root, &index_path, "test", dir.path()).unwrap();

    // Write and ingest a new page
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);
    let opts = IngestOptions { dry_run: false, ..Default::default() };
    ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();

    // Simulate what the caller (main.rs) does when auto_rebuild is true
    llm_wiki::search::rebuild_index(&wiki_root, &index_path, "test", dir.path()).unwrap();

    let status = llm_wiki::search::index_status("test", &index_path, dir.path()).unwrap();
    assert!(!status.stale);
    assert_eq!(status.pages, 1);

    // Search should find the page
    let results = llm_wiki::search::search(
        "Test Page",
        &llm_wiki::search::SearchOptions::default(),
        &index_path,
        "test",
        None,
    )
    .unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].slug, "concepts/foo");
}

#[test]
fn ingest_leaves_index_stale_when_auto_rebuild_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    let index_path = dir.path().join("index-store");

    // Build initial empty index
    llm_wiki::search::rebuild_index(&wiki_root, &index_path, "test", dir.path()).unwrap();

    // Write and ingest a new page (no rebuild — simulates auto_rebuild=false)
    write_page(&wiki_root, "concepts/bar.md", VALID_PAGE);
    let opts = IngestOptions { dry_run: false, auto_commit: true };
    ingest(
        Path::new("concepts/bar.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();

    // Index should be stale — ingest committed but caller did not rebuild
    let status = llm_wiki::search::index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale);
}

// ── normalize_line_endings ────────────────────────────────────────────────────

use llm_wiki::ingest::normalize_line_endings;

#[test]
fn normalize_line_endings_converts_crlf_to_lf() {
    assert_eq!(normalize_line_endings("a\r\nb\r\nc"), "a\nb\nc");
}

#[test]
fn normalize_line_endings_converts_lone_cr_to_lf() {
    assert_eq!(normalize_line_endings("a\rb\rc"), "a\nb\nc");
}

#[test]
fn normalize_line_endings_preserves_lf() {
    assert_eq!(normalize_line_endings("a\nb\nc"), "a\nb\nc");
}

#[test]
fn normalize_line_endings_handles_mixed() {
    assert_eq!(normalize_line_endings("a\r\nb\rc\nd"), "a\nb\nc\nd");
}

#[test]
fn ingest_normalizes_crlf_to_lf() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // Write a page with CRLF line endings
    let crlf_content = "---\r\ntitle: \"CRLF Test\"\r\nsummary: \"test\"\r\nstatus: active\r\nlast_updated: \"2025-01-01\"\r\ntype: concept\r\n---\r\n\r\n## Body\r\n";
    let page_path = wiki_root.join("concepts/crlf.md");
    std::fs::create_dir_all(page_path.parent().unwrap()).unwrap();
    std::fs::write(&page_path, crlf_content).unwrap();

    let opts = IngestOptions { dry_run: false, ..Default::default() };
    ingest(
        Path::new("concepts/crlf.md"),
        &opts,
        &wiki_root,
        &default_schema(),
        &default_validation(),
    )
    .unwrap();

    let result = std::fs::read_to_string(&page_path).unwrap();
    assert!(
        !result.contains('\r'),
        "file should have no CR after ingest"
    );
    assert!(result.contains("CRLF Test"));
    assert!(result.contains("## Body"));
}
