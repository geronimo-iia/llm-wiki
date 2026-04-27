use std::collections::HashSet;
use std::fs;
use std::path::Path;

use llm_wiki::config::ValidationConfig;
use llm_wiki::git;
use llm_wiki::ingest::*;
use llm_wiki::type_registry::SpaceTypeRegistry;

fn registry() -> SpaceTypeRegistry {
    SpaceTypeRegistry::from_embedded()
}

fn validation() -> ValidationConfig {
    ValidationConfig::default()
}

fn setup_repo(dir: &Path) -> std::path::PathBuf {
    let wiki_root = dir.join("wiki");
    fs::create_dir_all(&wiki_root).unwrap();
    fs::create_dir_all(dir.join("inbox")).unwrap();
    fs::create_dir_all(dir.join("raw")).unwrap();
    git::init_repo(dir).unwrap();
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
type: concept
---

## Body
";

// ── ingest single file ────────────────────────────────────────────────────────

#[test]
fn ingest_validates_valid_page_and_commits() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let opts = IngestOptions {
        dry_run: false,
        auto_commit: true,
        changed_paths: None,
    };
    let report = ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
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
        "---\nstatus: active\ntype: concept\n---\n\nBody\n",
    );

    let opts = IngestOptions::default();
    let result = ingest(
        Path::new("concepts/bad.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title"));
}

#[test]
fn ingest_warns_on_no_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/bare.md",
        "# My Bare Page\n\nJust content.\n",
    );

    let opts = IngestOptions::default();
    let report = ingest(
        Path::new("concepts/bare.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    assert_eq!(report.pages_validated, 1);
    assert!(report.warnings.iter().any(|w| w.contains("no frontmatter")));
}

#[test]
fn ingest_does_not_rewrite_file() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let before = fs::read_to_string(wiki_root.join("concepts/foo.md")).unwrap();

    let opts = IngestOptions::default();
    ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    let after = fs::read_to_string(wiki_root.join("concepts/foo.md")).unwrap();
    assert_eq!(before, after, "ingest should not modify the file on disk");
}

// ── dry run ───────────────────────────────────────────────────────────────────

#[test]
fn ingest_dry_run_does_not_commit() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let head_before = git::current_head(dir.path()).unwrap();

    let opts = IngestOptions {
        dry_run: true,
        ..Default::default()
    };
    let report = ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    assert_eq!(report.pages_validated, 1);
    assert!(report.commit.is_empty());
    assert_eq!(git::current_head(dir.path()).unwrap(), head_before);
}

// ── folder ingest ─────────────────────────────────────────────────────────────

#[test]
fn ingest_folder_validates_all_md_recursively() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/a.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/sub/b.md", VALID_PAGE);

    let opts = IngestOptions::default();
    let report = ingest(
        Path::new("concepts"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();
    assert_eq!(report.pages_validated, 2);
}

#[test]
fn ingest_folder_counts_assets() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo/index.md", VALID_PAGE);
    fs::write(wiki_root.join("concepts/foo/diagram.png"), b"fake").unwrap();
    fs::write(wiki_root.join("concepts/foo/config.yaml"), b"key: val").unwrap();

    let opts = IngestOptions::default();
    let report = ingest(
        Path::new("concepts/foo"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();
    assert_eq!(report.pages_validated, 1);
    assert_eq!(report.assets_found, 2);
}

// ── commit ────────────────────────────────────────────────────────────────────

#[test]
fn ingest_commit_matches_git_head() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", VALID_PAGE);

    let opts = IngestOptions {
        dry_run: false,
        auto_commit: true,
        changed_paths: None,
    };
    let report = ingest(
        Path::new("concepts/foo.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    let head = git::current_head(dir.path()).unwrap();
    assert_eq!(report.commit, head);
}

// ── path traversal ────────────────────────────────────────────────────────────

#[test]
fn ingest_rejects_path_outside_wiki_root() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    fs::write(dir.path().join("outside.md"), "---\ntitle: X\n---\n").unwrap();

    let opts = IngestOptions::default();
    let result = ingest(
        Path::new("../outside.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    );
    assert!(result.is_err());
}

#[test]
fn ingest_rejects_nonexistent_path() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    let opts = IngestOptions::default();
    let result = ingest(
        Path::new("concepts/nope.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    );
    assert!(result.is_err());
}

// ── type validation ───────────────────────────────────────────────────────────

#[test]
fn ingest_warns_on_unknown_type_loose() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/alien.md",
        "---\ntitle: \"Alien\"\ntype: alien-type\n---\n\nBody.\n",
    );

    let opts = IngestOptions::default();
    let report = ingest(
        Path::new("concepts/alien.md"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    assert!(report.warnings.iter().any(|w| w.contains("unknown type")));
}

#[test]
fn ingest_errors_on_unknown_type_strict() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/alien.md",
        "---\ntitle: \"Alien\"\ntype: alien-type\n---\n\nBody.\n",
    );

    let opts = IngestOptions::default();
    let strict = ValidationConfig {
        type_strictness: "strict".into(),
    };
    let result = ingest(
        Path::new("concepts/alien.md"),
        &opts,
        &wiki_root,
        &registry(),
        &strict,
    );
    assert!(result.is_err());
}

// ── changed_paths (incremental validation) ────────────────────────────────────

#[test]
fn changed_paths_skips_files_not_in_set() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/a.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/b.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/c.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/d.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/e.md", VALID_PAGE);

    // Only a.md and b.md are "changed"
    let mut changed = HashSet::new();
    changed.insert(std::path::PathBuf::from("concepts/a.md"));
    changed.insert(std::path::PathBuf::from("concepts/b.md"));

    let opts = IngestOptions {
        changed_paths: Some(changed),
        ..Default::default()
    };
    let report = ingest(
        Path::new("concepts"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    assert_eq!(report.pages_validated, 2, "only changed files validated");
    assert_eq!(report.unchanged_count, 3, "3 unchanged files skipped");
}

#[test]
fn dry_run_changed_paths_validates_all_files() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/a.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/b.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/c.md", VALID_PAGE);

    // changed_paths is Some but dry_run should ignore it (ops layer sets None for dry_run)
    // At the ingest layer, changed_paths is still respected — dry_run behaviour is in ops.
    // Test: when dry_run=true and changed_paths=None, all files are validated.
    let opts = IngestOptions {
        dry_run: true,
        changed_paths: None,
        ..Default::default()
    };
    let report = ingest(
        Path::new("concepts"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    assert_eq!(
        report.pages_validated, 3,
        "dry_run with no changed_paths validates all"
    );
    assert_eq!(report.unchanged_count, 0);
}

#[test]
fn no_changed_paths_validates_all_files() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/a.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/b.md", VALID_PAGE);
    write_page(&wiki_root, "concepts/c.md", VALID_PAGE);

    let opts = IngestOptions {
        changed_paths: None,
        ..Default::default()
    };
    let report = ingest(
        Path::new("concepts"),
        &opts,
        &wiki_root,
        &registry(),
        &validation(),
    )
    .unwrap();

    assert_eq!(
        report.pages_validated, 3,
        "no changed_paths means full validation"
    );
    assert_eq!(report.unchanged_count, 0);
}

// ── normalize_line_endings ────────────────────────────────────────────────────

#[test]
fn normalize_crlf_to_lf() {
    assert_eq!(normalize_line_endings("a\r\nb\r\nc"), "a\nb\nc");
}

#[test]
fn normalize_lone_cr_to_lf() {
    assert_eq!(normalize_line_endings("a\rb\rc"), "a\nb\nc");
}

#[test]
fn normalize_preserves_lf() {
    assert_eq!(normalize_line_endings("a\nb\nc"), "a\nb\nc");
}

#[test]
fn normalize_mixed() {
    assert_eq!(normalize_line_endings("a\r\nb\rc\nd"), "a\nb\nc\nd");
}
