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

    let opts = IngestOptions { dry_run: false };
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

    let opts = IngestOptions { dry_run: false };
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

    let opts = IngestOptions { dry_run: false };
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

    let opts = IngestOptions { dry_run: false };
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

    let opts = IngestOptions { dry_run: false };
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

    let opts = IngestOptions { dry_run: true };
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

    let opts = IngestOptions { dry_run: false };
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

    let opts = IngestOptions { dry_run: false };
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

    let opts = IngestOptions { dry_run: false };
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
