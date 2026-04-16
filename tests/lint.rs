use std::fs;
use std::path::Path;

use llm_wiki::config::{GlobalConfig, ResolvedConfig};
use llm_wiki::git;
use llm_wiki::lint::{lint, lint_fix, write_lint_md, LintReport};

fn default_resolved() -> ResolvedConfig {
    let global = GlobalConfig::default();
    llm_wiki::config::resolve(&global, &llm_wiki::config::WikiConfig::default())
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

fn concept_page(title: &str, sources: &[&str], concepts: &[&str], body: &str) -> String {
    let mut fm = format!(
        "---\ntitle: \"{title}\"\nsummary: \"A concept\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: concept\n"
    );
    if !sources.is_empty() {
        fm.push_str("sources:\n");
        for s in sources {
            fm.push_str(&format!("  - {s}\n"));
        }
    }
    if !concepts.is_empty() {
        fm.push_str("concepts:\n");
        for c in concepts {
            fm.push_str(&format!("  - {c}\n"));
        }
    }
    fm.push_str("---\n\n");
    fm.push_str(body);
    fm.push('\n');
    fm
}

fn simple_page(title: &str, page_type: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"s\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: {page_type}\n---\n\nBody.\n"
    )
}

// ── orphan detection ──────────────────────────────────────────────────────────

#[test]
fn lint_detects_orphan_pages_in_degree_0() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // Page A links to B, but C is orphaned (no one links to it)
    write_page(
        &wiki_root,
        "concepts/a.md",
        &concept_page("A", &[], &["concepts/b"], ""),
    );
    write_page(&wiki_root, "concepts/b.md", &simple_page("B", "concept"));
    write_page(&wiki_root, "concepts/c.md", &simple_page("C", "concept"));
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    let report = lint(&wiki_root, &resolved, "test").unwrap();

    let orphan_slugs: Vec<&str> = report.orphans.iter().map(|o| o.slug.as_str()).collect();
    // A and C are orphans (no incoming links), B has incoming from A
    assert!(
        orphan_slugs.contains(&"concepts/a"),
        "A should be orphan: {orphan_slugs:?}"
    );
    assert!(
        orphan_slugs.contains(&"concepts/c"),
        "C should be orphan: {orphan_slugs:?}"
    );
}

#[test]
fn lint_does_not_flag_pages_with_incoming_link_as_orphans() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &concept_page("A", &[], &["concepts/b"], ""),
    );
    write_page(&wiki_root, "concepts/b.md", &simple_page("B", "concept"));
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    let report = lint(&wiki_root, &resolved, "test").unwrap();

    let orphan_slugs: Vec<&str> = report.orphans.iter().map(|o| o.slug.as_str()).collect();
    assert!(
        !orphan_slugs.contains(&"concepts/b"),
        "B has incoming link from A, should not be orphan"
    );
}

// ── missing stubs ─────────────────────────────────────────────────────────────

#[test]
fn lint_detects_missing_stubs() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &concept_page(
            "A",
            &["sources/nonexistent"],
            &[],
            "See [[concepts/also-missing]].",
        ),
    );
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    let report = lint(&wiki_root, &resolved, "test").unwrap();

    assert!(report
        .missing_stubs
        .contains(&"sources/nonexistent".to_string()));
    assert!(report
        .missing_stubs
        .contains(&"concepts/also-missing".to_string()));
}

// ── empty sections ────────────────────────────────────────────────────────────

#[test]
fn lint_detects_empty_sections() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // Create a directory with a .md file but no index.md
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &simple_page("Foo", "concept"),
    );
    // concepts/ dir exists but has no index.md
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    let report = lint(&wiki_root, &resolved, "test").unwrap();

    assert!(
        report.empty_sections.contains(&"concepts".to_string()),
        "concepts/ should be empty section: {:?}",
        report.empty_sections
    );
}

// ── untyped sources ───────────────────────────────────────────────────────────

#[test]
fn lint_detects_untyped_sources_source_summary_type() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "sources/old.md",
        &simple_page("Old Source", "source-summary"),
    );
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    let report = lint(&wiki_root, &resolved, "test").unwrap();

    assert!(
        report.untyped_sources.contains(&"sources/old".to_string()),
        "source-summary should be flagged: {:?}",
        report.untyped_sources
    );
}

#[test]
fn lint_detects_untyped_sources_missing_type_on_source_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "sources/untyped.md",
        &simple_page("Untyped", "page"),
    );
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    let report = lint(&wiki_root, &resolved, "test").unwrap();

    assert!(
        report
            .untyped_sources
            .contains(&"sources/untyped".to_string()),
        "sources/ page with type 'page' should be flagged: {:?}",
        report.untyped_sources
    );
}

// ── write_lint_md ─────────────────────────────────────────────────────────────

#[test]
fn write_lint_md_writes_all_5_sections_even_when_empty() {
    let dir = tempfile::tempdir().unwrap();
    let report = LintReport {
        orphans: Vec::new(),
        missing_stubs: Vec::new(),
        empty_sections: Vec::new(),
        missing_connections: Vec::new(),
        untyped_sources: Vec::new(),
        date: "2025-07-15".to_string(),
    };

    write_lint_md(&report, dir.path()).unwrap();

    let content = fs::read_to_string(dir.path().join("LINT.md")).unwrap();
    assert!(content.contains("## Orphans (0)"));
    assert!(content.contains("## Missing Stubs (0)"));
    assert!(content.contains("## Empty Sections (0)"));
    assert!(content.contains("## Missing Connections (0)"));
    assert!(content.contains("## Untyped Sources (0)"));
}

#[test]
fn write_lint_md_shows_no_x_found_for_empty_sections() {
    let dir = tempfile::tempdir().unwrap();
    let report = LintReport {
        orphans: Vec::new(),
        missing_stubs: Vec::new(),
        empty_sections: Vec::new(),
        missing_connections: Vec::new(),
        untyped_sources: Vec::new(),
        date: "2025-07-15".to_string(),
    };

    write_lint_md(&report, dir.path()).unwrap();

    let content = fs::read_to_string(dir.path().join("LINT.md")).unwrap();
    assert!(content.contains("_No orphans found._"));
    assert!(content.contains("_No missing stubs found._"));
    assert!(content.contains("_No empty sections found._"));
    assert!(content.contains("_No missing connections found._"));
    assert!(content.contains("_No untyped sources found._"));
}

// ── lint_fix ──────────────────────────────────────────────────────────────────

#[test]
fn lint_fix_creates_stub_pages_for_missing_stubs() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &concept_page("A", &["sources/missing-paper"], &[], ""),
    );
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    lint_fix(&wiki_root, &resolved, None, "test").unwrap();

    // The stub should now exist
    assert!(
        wiki_root.join("sources/missing-paper.md").exists(),
        "stub page should have been created"
    );
}

#[test]
fn lint_fix_creates_index_md_for_empty_sections() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/foo.md",
        &simple_page("Foo", "concept"),
    );
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    lint_fix(&wiki_root, &resolved, None, "test").unwrap();

    assert!(
        wiki_root.join("concepts/index.md").exists(),
        "section index should have been created"
    );
}

#[test]
fn lint_fix_with_only_missing_stubs_does_not_touch_empty_sections() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // Create an empty section and a missing stub reference
    write_page(
        &wiki_root,
        "concepts/a.md",
        &concept_page("A", &["sources/missing"], &[], ""),
    );
    git::commit(dir.path(), "add pages").unwrap();

    let resolved = default_resolved();
    lint_fix(&wiki_root, &resolved, Some("missing-stubs"), "test").unwrap();

    // Stub should be created
    assert!(wiki_root.join("sources/missing.md").exists());
    // But empty section should NOT be fixed
    assert!(
        !wiki_root.join("concepts/index.md").exists(),
        "empty section should not be fixed when only=missing-stubs"
    );
}
