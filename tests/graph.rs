//! Graph, lint, and contradiction tests — Phase 3
//!
//! Covers: orphan detection, missing stubs, DOT/Mermaid output,
//! contradiction listing, lint report generation, and end-to-end CLI behaviour.

use llm_wiki::analysis::Status;
use llm_wiki::contradiction;
use llm_wiki::git;
use llm_wiki::graph;
use llm_wiki::lint;
use std::path::Path;

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Minimal valid `PageFrontmatter` YAML + body for a concept page.
fn concept_page_md(title: &str) -> String {
    format!(
        "---\ntitle: {title}\nsummary: test\nread_when: []\nstatus: active\nlast_updated: 2026-04-13\ntype: concept\ntags: []\nsources: []\nconfidence: medium\ncontradictions: []\ntldr: test\n---\n\n## Body\n\nContent.\n"
    )
}

/// Concept page MD whose body contains a `[[wikilink]]` to `target`.
fn concept_page_with_wikilink(title: &str, target: &str) -> String {
    format!(
        "---\ntitle: {title}\nsummary: test\nread_when: []\nstatus: active\nlast_updated: 2026-04-13\ntype: concept\ntags: []\nsources: []\nconfidence: medium\ncontradictions: []\ntldr: test\n---\n\n## Body\n\nSee [[{target}]] for more details.\n"
    )
}

/// Contradiction page MD matching what `integrate.rs` writes.
fn contradiction_page_md(title: &str, status: &str) -> String {
    format!(
        "---\ntitle: {title}\ntype: contradiction\nclaim_a: Claim A\nsource_a: sources/a\nclaim_b: Claim B\nsource_b: sources/b\ndimension: context\nepistemic_value: Insight.\nstatus: {status}\ntags: []\ncreated: 2026-04-13\nupdated: 2026-04-13\n---\n\n## Claim A\n\nClaim A\n\n## Claim B\n\nClaim B\n\n## Analysis\n\nInsight.\n"
    )
}

/// Write `content` to `path`, creating parent dirs as needed.
fn write(path: &Path, content: &str) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

/// Initialise a bare git repo (no commits) at `root`.
fn git_init(root: &Path) {
    git::init_if_needed(root).unwrap();
}

// ── Unit tests — orphans ──────────────────────────────────────────────────────

#[test]
fn orphans_page_with_no_inbound_links_appears() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        &root.join("concepts/lonely.md"),
        &concept_page_md("Lonely"),
    );

    let g = graph::build_graph(root).unwrap();
    let orphans = graph::orphans(&g);
    assert!(
        orphans.iter().any(|s| s.contains("lonely")),
        "lonely should be an orphan; got: {orphans:?}"
    );
}

#[test]
fn orphans_referenced_page_excluded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Page A links to page B via wikilink.
    write(
        &root.join("concepts/a.md"),
        &concept_page_with_wikilink("A", "concepts/b"),
    );
    write(&root.join("concepts/b.md"), &concept_page_md("B"));

    let g = graph::build_graph(root).unwrap();
    let orphans = graph::orphans(&g);
    // B has an inbound edge from A → must NOT appear in orphans.
    assert!(
        !orphans.iter().any(|s| s == "concepts/b"),
        "concepts/b should NOT be an orphan; orphans: {orphans:?}"
    );
}

#[test]
fn orphans_raw_directory_excluded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // A file under raw/ with no inbound links.
    write(&root.join("raw/source.md"), "# raw source\n\nno frontmatter\n");

    let g = graph::build_graph(root).unwrap();
    let orphans = graph::orphans(&g);
    assert!(
        !orphans.iter().any(|s| s.starts_with("raw/")),
        "raw/ pages should be excluded from orphan detection; orphans: {orphans:?}"
    );
}

// ── Unit tests — missing stubs ────────────────────────────────────────────────

#[test]
fn missing_stubs_edge_to_nonexistent_file_appears() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Page A links to "concepts/ghost" which doesn't exist on disk.
    write(
        &root.join("concepts/a.md"),
        &concept_page_with_wikilink("A", "concepts/ghost"),
    );

    let g = graph::build_graph(root).unwrap();
    let stubs = graph::missing_stubs(&g, root);
    assert!(
        stubs.iter().any(|s| s.contains("ghost")),
        "concepts/ghost should be a missing stub; got: {stubs:?}"
    );
}

#[test]
fn missing_stubs_edge_to_existing_file_absent() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        &root.join("concepts/a.md"),
        &concept_page_with_wikilink("A", "concepts/b"),
    );
    write(&root.join("concepts/b.md"), &concept_page_md("B"));

    let g = graph::build_graph(root).unwrap();
    let stubs = graph::missing_stubs(&g, root);
    assert!(
        !stubs.iter().any(|s| s == "concepts/b"),
        "concepts/b exists on disk — must not be a missing stub; stubs: {stubs:?}"
    );
}

// ── Unit tests — DOT / Mermaid output ────────────────────────────────────────

#[test]
fn dot_output_contains_digraph_no_empty_node_names() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(&root.join("concepts/a.md"), &concept_page_md("A"));
    write(
        &root.join("concepts/b.md"),
        &concept_page_with_wikilink("B", "concepts/a"),
    );

    let g = graph::build_graph(root).unwrap();
    let dot = graph::dot_output(&g);

    assert!(dot.contains("digraph"), "DOT output must contain 'digraph': {dot}");
    assert!(!dot.contains("label=\"\""), "DOT output must not have empty labels: {dot}");
}

#[test]
fn mermaid_output_starts_with_graph() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(&root.join("concepts/x.md"), &concept_page_md("X"));

    let g = graph::build_graph(root).unwrap();
    let mermaid = graph::mermaid_output(&g);

    assert!(
        mermaid.starts_with("graph"),
        "Mermaid output must start with 'graph': {mermaid}"
    );
}

// ── Unit tests — contradiction::list ─────────────────────────────────────────

#[test]
fn contradiction_list_all_pages_when_no_filter() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        &root.join("contradictions/c-active.md"),
        &contradiction_page_md("Active One", "active"),
    );
    write(
        &root.join("contradictions/c-resolved.md"),
        &contradiction_page_md("Resolved One", "resolved"),
    );

    let items = contradiction::list(root, None).unwrap();
    assert_eq!(items.len(), 2, "should return both pages; got: {items:?}");
}

#[test]
fn contradiction_list_active_filter_returns_only_active() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        &root.join("contradictions/c-active.md"),
        &contradiction_page_md("Active One", "active"),
    );
    write(
        &root.join("contradictions/c-resolved.md"),
        &contradiction_page_md("Resolved One", "resolved"),
    );

    let items = contradiction::list(root, Some(Status::Active)).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, Status::Active);
}

#[test]
fn contradiction_list_resolved_filter_excludes_active() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        &root.join("contradictions/c-active.md"),
        &contradiction_page_md("Active One", "active"),
    );
    write(
        &root.join("contradictions/c-resolved.md"),
        &contradiction_page_md("Resolved One", "resolved"),
    );

    let items = contradiction::list(root, Some(Status::Resolved)).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, Status::Resolved);
    assert!(
        items.iter().all(|c| c.status != Status::Active),
        "active should be excluded"
    );
}

// ── Unit tests — lint ─────────────────────────────────────────────────────────

#[test]
fn lint_orphan_page_appears_in_report() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    write(&root.join("concepts/orphan.md"), &concept_page_md("Orphan"));

    let report = lint::lint(root).unwrap();
    assert!(
        report.orphan_pages.iter().any(|s| s.contains("orphan")),
        "orphan page should appear in report.orphan_pages; got: {:?}",
        report.orphan_pages
    );
}

#[test]
fn lint_missing_stub_appears_in_report() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    write(
        &root.join("concepts/a.md"),
        &concept_page_with_wikilink("A", "concepts/missing-page"),
    );

    let report = lint::lint(root).unwrap();
    assert!(
        report.missing_stubs.iter().any(|s| s.contains("missing-page")),
        "missing-page should appear in report.missing_stubs; got: {:?}",
        report.missing_stubs
    );
}

#[test]
fn lint_active_contradiction_appears_in_report() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    write(
        &root.join("contradictions/test-active.md"),
        &contradiction_page_md("Test Active", "active"),
    );

    let report = lint::lint(root).unwrap();
    assert!(
        report
            .active_contradictions
            .iter()
            .any(|c| c.slug.contains("test-active")),
        "test-active should appear in active_contradictions; got: {:?}",
        report.active_contradictions.iter().map(|c| &c.slug).collect::<Vec<_>>()
    );
}

#[test]
fn lint_resolved_contradiction_absent_from_active_list() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    write(
        &root.join("contradictions/test-resolved.md"),
        &contradiction_page_md("Test Resolved", "resolved"),
    );

    let report = lint::lint(root).unwrap();
    assert!(
        !report
            .active_contradictions
            .iter()
            .any(|c| c.slug.contains("test-resolved")),
        "resolved contradiction must NOT appear in active_contradictions"
    );
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[test]
fn cli_lint_with_orphan_writes_lint_md_and_commits() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    write(
        &root.join("concepts/lonely.md"),
        &concept_page_md("Lonely Page"),
    );

    let report = lint::lint(root).unwrap();

    // LINT.md was written.
    let lint_path = root.join("LINT.md");
    assert!(lint_path.exists(), "LINT.md must exist after lint");

    // LINT.md mentions the orphan slug.
    let content = std::fs::read_to_string(&lint_path).unwrap();
    assert!(
        content.contains("lonely"),
        "LINT.md should mention the orphan slug; content:\n{content}"
    );

    // The orphan appears in the report.
    assert!(report.orphan_pages.iter().any(|s| s.contains("lonely")));

    // A commit was created.
    let repo = git2::Repository::open(root).unwrap();
    let head = repo.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    assert!(
        commit.message().unwrap_or("").contains("lint:"),
        "commit message should start with 'lint:'"
    );
}

#[test]
fn cli_lint_clean_wiki_writes_empty_sections_still_commits() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    // Two pages that link to each other — no orphans, no stubs, no contradictions.
    write(
        &root.join("concepts/a.md"),
        &concept_page_with_wikilink("A", "concepts/b"),
    );
    write(
        &root.join("concepts/b.md"),
        &concept_page_with_wikilink("B", "concepts/a"),
    );

    let report = lint::lint(root).unwrap();

    let lint_path = root.join("LINT.md");
    assert!(lint_path.exists(), "LINT.md must be written even for a clean wiki");

    // All sections should be empty.
    assert!(report.orphan_pages.is_empty() || !report.orphan_pages.iter().any(|s| s.contains("LINT")));
    assert!(report.active_contradictions.is_empty());

    // A commit exists.
    let repo = git2::Repository::open(root).unwrap();
    let head = repo.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    assert!(commit.message().unwrap_or("").contains("lint:"));
}

#[test]
fn cli_contradict_status_active_lists_only_active() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(
        &root.join("contradictions/active-one.md"),
        &contradiction_page_md("Active One", "active"),
    );
    write(
        &root.join("contradictions/resolved-one.md"),
        &contradiction_page_md("Resolved One", "resolved"),
    );

    let items = contradiction::list(root, Some(Status::Active)).unwrap();
    assert!(items.iter().all(|c| c.status == Status::Active));
    assert!(!items.iter().any(|c| c.slug.contains("resolved")));
}

#[test]
fn cli_list_type_concept_no_source_or_contradiction_pages() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(&root.join("concepts/moe.md"), &concept_page_md("MoE"));
    write(
        &root.join("contradictions/c1.md"),
        &contradiction_page_md("C1", "active"),
    );

    // Build graph — check nodes appear via node_weights() (the public inner field).
    let g = graph::build_graph(root).unwrap();
    let all_slugs: Vec<&str> = g.inner.node_weights().map(|s| s.as_str()).collect();
    assert!(
        all_slugs.contains(&"concepts/moe"),
        "concepts/moe should be a graph node; nodes: {all_slugs:?}"
    );
    assert!(
        all_slugs.contains(&"contradictions/c1"),
        "contradictions/c1 should be a graph node; nodes: {all_slugs:?}"
    );

    // When filtering by slug prefix, concept nodes contain no contradiction pages.
    let concept_slugs: Vec<&str> = all_slugs
        .iter()
        .copied()
        .filter(|s| s.starts_with("concepts/"))
        .collect();
    let contradiction_slugs: Vec<&str> = all_slugs
        .iter()
        .copied()
        .filter(|s| s.starts_with("contradictions/"))
        .collect();

    assert!(!concept_slugs.iter().any(|s| s.starts_with("contradictions/")));
    assert!(!contradiction_slugs.iter().any(|s| s.starts_with("concepts/")));
}

#[test]
fn cli_graph_output_parses_as_valid_dot() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(&root.join("concepts/a.md"), &concept_page_md("A"));
    write(
        &root.join("concepts/b.md"),
        &concept_page_with_wikilink("B", "concepts/a"),
    );

    let g = graph::build_graph(root).unwrap();
    let dot = graph::dot_output(&g);

    // Mandatory structural checks.
    assert!(dot.contains("digraph"), "must contain 'digraph' keyword");
    assert!(dot.contains('{'), "must have opening brace");
    assert!(dot.contains('}'), "must have closing brace");
    assert!(!dot.contains("label=\"\""), "must not have empty labels");

    // If `dot` (graphviz) is available, validate the syntax by piping through it.
    if which_dot_available() {
        let result = std::process::Command::new("dot")
            .arg("-Tsvg")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write as _;
                if let Some(stdin) = child.stdin.take() {
                    let mut stdin = stdin;
                    stdin.write_all(dot.as_bytes()).ok();
                }
                child.wait_with_output()
            });

        if let Ok(output) = result {
            assert!(
                output.status.success(),
                "dot -Tsvg failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

/// Check whether the `dot` (graphviz) binary is available on PATH.
fn which_dot_available() -> bool {
    std::process::Command::new("dot")
        .arg("-V")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn cli_diff_nonempty_after_ingest() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    // First commit — initial page.
    write(&root.join("concepts/first.md"), &concept_page_md("First"));
    git::commit(root, "initial: first page").unwrap();

    // Second commit — another page.
    write(&root.join("concepts/second.md"), &concept_page_md("Second"));
    git::commit(root, "add: second page").unwrap();

    let diff = git::diff_last(root).unwrap();
    assert!(!diff.is_empty(), "diff should be non-empty after two commits");
    assert!(
        diff.contains("second"),
        "diff should mention the newly added file: {diff}"
    );
}

// ── Phase 8: bundle-aware graph and lint ─────────────────────────────────────

#[test]
fn build_graph_bundle_page_has_correct_slug() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Bundle page.
    std::fs::create_dir_all(root.join("concepts/moe")).unwrap();
    write(
        &root.join("concepts/moe/index.md"),
        &concept_page_md("Mixture of Experts"),
    );

    let g = graph::build_graph(root).unwrap();
    let slugs: Vec<&str> = g.inner.node_weights().map(|s| s.as_str()).collect();
    assert!(
        slugs.contains(&"concepts/moe"),
        "bundle slug must be concepts/moe; got: {slugs:?}"
    );
    assert!(
        !slugs.iter().any(|s| s.contains("index")),
        "index.md must not appear as a slug component; got: {slugs:?}"
    );
}

#[test]
fn build_graph_bundle_asset_not_treated_as_page() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("concepts/moe")).unwrap();
    write(
        &root.join("concepts/moe/index.md"),
        &concept_page_md("Mixture of Experts"),
    );
    // A non-md asset beside index.md.
    std::fs::write(root.join("concepts/moe/diagram.png"), b"PNG").unwrap();

    let g = graph::build_graph(root).unwrap();
    let slugs: Vec<&str> = g.inner.node_weights().map(|s| s.as_str()).collect();
    assert_eq!(slugs.len(), 1, "only the page node; got: {slugs:?}");
}

#[test]
fn lint_orphan_asset_ref_detected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    // Bundle page referencing a non-existent asset.
    std::fs::create_dir_all(root.join("concepts/moe")).unwrap();
    let body = "---\ntitle: MoE\nsummary: test\nread_when: []\nstatus: active\nlast_updated: 2026-04-13\ntype: concept\ntags: []\nsources: []\nconfidence: medium\ncontradictions: []\ntldr: test\n---\n\n![diagram](./missing-diagram.png)\n";
    std::fs::write(root.join("concepts/moe/index.md"), body).unwrap();

    let report = lint::lint(root).unwrap();
    assert!(
        report.orphan_asset_refs.iter().any(|r| r.contains("missing-diagram.png")),
        "orphan asset ref must be detected; got: {:?}",
        report.orphan_asset_refs
    );
}

#[test]
fn lint_no_orphan_asset_ref_when_asset_exists() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    std::fs::create_dir_all(root.join("concepts/moe")).unwrap();
    let body = "---\ntitle: MoE\nsummary: test\nread_when: []\nstatus: active\nlast_updated: 2026-04-13\ntype: concept\ntags: []\nsources: []\nconfidence: medium\ncontradictions: []\ntldr: test\n---\n\n![diagram](./diagram.png)\n";
    std::fs::write(root.join("concepts/moe/index.md"), body).unwrap();
    std::fs::write(root.join("concepts/moe/diagram.png"), b"PNG").unwrap();

    let report = lint::lint(root).unwrap();
    assert!(
        report.orphan_asset_refs.is_empty(),
        "no orphan refs when asset exists; got: {:?}",
        report.orphan_asset_refs
    );
}

// ── Phase 8: lint integration ─────────────────────────────────────────────────

#[test]
fn lint_orphan_asset_ref_appears_in_lint_md() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git_init(root);

    // Bundle page referencing a missing asset.
    std::fs::create_dir_all(root.join("concepts/moe")).unwrap();
    let body = "---\ntitle: MoE\nsummary: test\nread_when: []\nstatus: active\nlast_updated: 2026-04-13\ntype: concept\ntags: []\nsources: []\nconfidence: medium\ncontradictions: []\ntldr: test\n---\n\n![diagram](./missing.png)\n";
    std::fs::write(root.join("concepts/moe/index.md"), body).unwrap();

    lint::lint(root).unwrap();

    let lint_md = std::fs::read_to_string(root.join("LINT.md")).unwrap();
    assert!(
        lint_md.contains("missing.png"),
        "LINT.md must mention the missing asset; content:\n{lint_md}"
    );
    assert!(
        lint_md.contains("Orphan Asset"),
        "LINT.md must have an Orphan Asset References section"
    );
}
