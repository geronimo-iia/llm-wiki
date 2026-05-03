use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use llm_wiki::config::GlobalConfig;
use llm_wiki::engine::{EngineState, SpaceContext};
use llm_wiki::git;
use llm_wiki::index_manager::SpaceIndexManager;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::ops::{LintFinding, Severity, run_lint};
use llm_wiki::space_builder;
use llm_wiki::type_registry::SpaceTypeRegistry;

fn schema() -> IndexSchema {
    let (_registry, schema) = space_builder::build_space_from_embedded("en_stem");
    schema
}

fn registry() -> SpaceTypeRegistry {
    let (registry, _schema) = space_builder::build_space_from_embedded("en_stem");
    registry
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

fn build_engine(dir: &Path, wiki_root: &Path) -> EngineState {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    let mgr = SpaceIndexManager::new("test", &index_path);
    mgr.rebuild(wiki_root, dir, &schema(), &registry()).unwrap();
    mgr.open(&schema(), None).unwrap();

    let space = Arc::new(SpaceContext {
        name: "test".to_string(),
        wiki_root: wiki_root.to_path_buf(),
        repo_root: dir.to_path_buf(),
        type_registry: registry(),
        index_schema: schema(),
        index_manager: mgr,
        graph_cache: petgraph_live::cache::GenerationCache::new(),
        community_cache: petgraph_live::cache::GenerationCache::new(),
    });

    let mut spaces = HashMap::new();
    spaces.insert("test".to_string(), space);

    EngineState {
        config: GlobalConfig::default(),
        config_path: dir.join("config.toml"),
        state_dir: dir.to_path_buf(),
        spaces,
    }
}

fn findings_for_rule<'a>(findings: &'a [LintFinding], rule: &str) -> Vec<&'a LintFinding> {
    findings.iter().filter(|f| f.rule == rule).collect()
}

fn slugs_for_rule(findings: &[LintFinding], rule: &str) -> Vec<String> {
    findings_for_rule(findings, rule)
        .into_iter()
        .map(|f| f.slug.clone())
        .collect()
}

// ── orphan ────────────────────────────────────────────────────────────────────

#[test]
fn orphan_detects_unlinked_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // linked.md links to orphan.md, so orphan.md has an incoming link
    // linked.md itself has no incoming link → it is the orphan
    write_page(
        &wiki_root,
        "concepts/linked.md",
        "---\ntitle: \"Linked\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/orphan]].\n",
    );
    write_page(
        &wiki_root,
        "concepts/orphan.md",
        "---\ntitle: \"Orphan\"\ntype: concept\nread_when: [\"x\"]\n---\n\nNo one links here.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("orphan"), None).unwrap();
    let slugs = slugs_for_rule(&report.findings, "orphan");

    assert!(
        slugs.contains(&"concepts/linked".to_string()),
        "linked.md has no incoming links so it is the orphan: {slugs:?}"
    );
    assert!(
        !slugs.contains(&"concepts/orphan".to_string()),
        "orphan.md is linked to by linked.md, so it is not an orphan: {slugs:?}"
    );
}

#[test]
fn orphan_ignores_section_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/index.md",
        "---\ntitle: \"Concepts\"\ntype: section\n---\n\nSection root.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("orphan"), None).unwrap();
    let slugs = slugs_for_rule(&report.findings, "orphan");

    assert!(
        !slugs.contains(&"concepts/index".to_string()),
        "section pages should not be flagged as orphans"
    );
    assert!(
        !slugs.contains(&"concepts".to_string()),
        "section index pages should not be flagged as orphans"
    );
}

// ── broken-link ───────────────────────────────────────────────────────────────

#[test]
fn broken_link_detects_missing_slug_in_body_links() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/alpha.md",
        "---\ntitle: \"Alpha\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/does-not-exist]] for details.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("broken-link"), None).unwrap();
    let findings = findings_for_rule(&report.findings, "broken-link");

    assert!(!findings.is_empty(), "should detect broken link");
    assert_eq!(findings[0].slug, "concepts/alpha");
    assert!(
        findings[0].message.contains("concepts/does-not-exist"),
        "message should name the missing slug: {}",
        findings[0].message
    );
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn broken_link_clean_when_all_slugs_exist() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/alpha.md",
        "---\ntitle: \"Alpha\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/beta]].\n",
    );
    write_page(
        &wiki_root,
        "concepts/beta.md",
        "---\ntitle: \"Beta\"\ntype: concept\nread_when: [\"x\"]\n---\n\nExists.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("broken-link"), None).unwrap();
    let findings = findings_for_rule(&report.findings, "broken-link");

    assert!(
        findings.is_empty(),
        "no broken links expected: {findings:?}"
    );
}

// ── unknown-type ──────────────────────────────────────────────────────────────

#[test]
fn unknown_type_flags_unregistered_type() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "things/widget.md",
        "---\ntitle: \"Widget\"\ntype: widget\n---\n\nA widget.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("unknown-type"), None).unwrap();
    let findings = findings_for_rule(&report.findings, "unknown-type");

    assert!(!findings.is_empty(), "unknown type should be flagged");
    assert_eq!(findings[0].slug, "things/widget");
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn unknown_type_clean_for_known_types() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/moe.md",
        "---\ntitle: \"MoE\"\ntype: concept\nread_when: [\"x\"]\n---\n\nContent.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("unknown-type"), None).unwrap();
    let findings = findings_for_rule(&report.findings, "unknown-type");

    assert!(
        findings.is_empty(),
        "known type should not be flagged: {findings:?}"
    );
}

// ── stale ─────────────────────────────────────────────────────────────────────

#[test]
fn stale_old_page_low_confidence_is_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/old.md",
        "---\ntitle: \"Old\"\ntype: concept\nread_when: [\"x\"]\nlast_updated: \"2020-01-01\"\nconfidence: 0.2\n---\n\nOld content.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("stale"), None).unwrap();
    let findings = findings_for_rule(&report.findings, "stale");

    assert!(!findings.is_empty(), "old + low confidence should be stale");
    assert_eq!(findings[0].slug, "concepts/old");
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn stale_old_page_high_confidence_not_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/trusted.md",
        "---\ntitle: \"Trusted\"\ntype: concept\nread_when: [\"x\"]\nlast_updated: \"2020-01-01\"\nconfidence: 0.9\n---\n\nWell-verified content.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("stale"), None).unwrap();
    let findings = findings_for_rule(&report.findings, "stale");

    assert!(
        findings.is_empty(),
        "old but high-confidence should NOT be stale: {findings:?}"
    );
}

#[test]
fn stale_recent_page_not_flagged_regardless_of_confidence() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    let recent = chrono::Utc::now().format("%Y-%m-%d").to_string();
    write_page(
        &wiki_root,
        "concepts/shaky.md",
        &format!(
            "---\ntitle: \"Shaky\"\ntype: concept\nread_when: [\"x\"]\nlast_updated: \"{recent}\"\nconfidence: 0.1\n---\n\nSpeculative.\n"
        ),
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("stale"), None).unwrap();
    let findings = findings_for_rule(&report.findings, "stale");

    // Both conditions (old AND low confidence) must hold — recent date means not stale
    assert!(
        findings.is_empty(),
        "recent date means stale condition not met: {findings:?}"
    );
}

// ── severity filter ───────────────────────────────────────────────────────────

#[test]
fn severity_filter_returns_only_errors() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/alpha.md",
        "---\ntitle: \"Alpha\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/missing]].\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", None, Some("error")).unwrap();

    for f in &report.findings {
        assert_eq!(
            f.severity,
            Severity::Error,
            "only errors expected, got: {f:?}"
        );
    }
}

// ── integration: known-bad wiki ───────────────────────────────────────────────

#[test]
fn integration_known_bad_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // broken link
    write_page(
        &wiki_root,
        "concepts/broken.md",
        "---\ntitle: \"Broken\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/ghost]].\n",
    );
    // unknown type
    write_page(
        &wiki_root,
        "things/widget.md",
        "---\ntitle: \"Widget\"\ntype: widget\n---\n\nA widget.\n",
    );
    // clean page that links to broken.md (gives it an incoming link)
    write_page(
        &wiki_root,
        "concepts/clean.md",
        "---\ntitle: \"Clean\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/broken]].\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", None, None).unwrap();

    let broken_link_slugs = slugs_for_rule(&report.findings, "broken-link");
    let unknown_type_slugs = slugs_for_rule(&report.findings, "unknown-type");

    assert!(
        broken_link_slugs.contains(&"concepts/broken".to_string()),
        "broken link should be flagged: {broken_link_slugs:?}"
    );
    assert!(
        unknown_type_slugs.contains(&"things/widget".to_string()),
        "unknown type should be flagged: {unknown_type_slugs:?}"
    );
    assert!(report.errors > 0, "should have errors");
}

// ── broken-cross-wiki-link ────────────────────────────────────────────────────

fn build_engine_with_name(dir: &Path, wiki_root: &Path, name: &str) -> EngineState {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    let mgr = SpaceIndexManager::new(name, &index_path);
    mgr.rebuild(wiki_root, dir, &schema(), &registry()).unwrap();
    mgr.open(&schema(), None).unwrap();

    let space = Arc::new(SpaceContext {
        name: name.to_string(),
        wiki_root: wiki_root.to_path_buf(),
        repo_root: dir.to_path_buf(),
        type_registry: registry(),
        index_schema: schema(),
        index_manager: mgr,
        graph_cache: petgraph_live::cache::GenerationCache::new(),
        community_cache: petgraph_live::cache::GenerationCache::new(),
    });

    let mut spaces = HashMap::new();
    spaces.insert(name.to_string(), space);

    EngineState {
        config: GlobalConfig::default(),
        config_path: dir.join("config.toml"),
        state_dir: dir.to_path_buf(),
        spaces,
    }
}

#[test]
fn broken_cross_wiki_link_to_unmounted_wiki_is_warning() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/a.md",
        "---\ntitle: \"A\"\ntype: concept\nread_when: [\"x\"]\nsources:\n  - wiki://unmounted/concepts/b\n---\n\nBody.\n",
    );

    let engine = build_engine_with_name(dir.path(), &wiki_root, "mywiki");
    let report = run_lint(&engine, "mywiki", Some("broken-cross-wiki-link"), None).unwrap();

    let cross_wiki_findings = slugs_for_rule(&report.findings, "broken-cross-wiki-link");
    assert!(
        cross_wiki_findings.contains(&"concepts/a".to_string()),
        "should flag cross-wiki link to unmounted wiki: {cross_wiki_findings:?}"
    );
    // Should be Warning severity, not Error
    let finding = report
        .findings
        .iter()
        .find(|f| f.rule == "broken-cross-wiki-link")
        .unwrap();
    assert_eq!(finding.severity, Severity::Warning);
}

#[test]
fn broken_cross_wiki_link_to_mounted_wiki_no_finding() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/a.md",
        "---\ntitle: \"A\"\ntype: concept\nread_when: [\"x\"]\nsources:\n  - wiki://mywiki/concepts/local\n---\n\nBody.\n",
    );
    // Provide a second page so there's an incoming link to concepts/a
    write_page(
        &wiki_root,
        "concepts/b.md",
        "---\ntitle: \"B\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/a]].\n",
    );

    // Engine named "mywiki" — the wiki:// link targets "mywiki" which IS mounted
    let engine = build_engine_with_name(dir.path(), &wiki_root, "mywiki");
    let report = run_lint(&engine, "mywiki", Some("broken-cross-wiki-link"), None).unwrap();

    let cross_wiki_findings = slugs_for_rule(&report.findings, "broken-cross-wiki-link");
    assert!(
        cross_wiki_findings.is_empty(),
        "mounted wiki should not produce findings: {cross_wiki_findings:?}"
    );
}

// ── LintFinding.path ──────────────────────────────────────────────────────────

#[test]
fn lint_finding_path_is_populated() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/orphan.md",
        "---\ntitle: \"Orphan\"\ntype: concept\nread_when: [\"x\"]\n---\n\nNo one links here.\n",
    );

    let engine = build_engine(dir.path(), &wiki_root);
    let report = run_lint(&engine, "test", Some("orphan"), None).unwrap();

    let findings = findings_for_rule(&report.findings, "orphan");
    assert!(!findings.is_empty(), "expected at least one orphan finding");

    for f in &findings {
        assert!(
            !f.path.is_empty(),
            "path should be non-empty for finding: {:?}",
            f.slug
        );
        assert!(
            f.path.ends_with(".md"),
            "path should end with .md: {}",
            f.path
        );
        assert!(
            f.path.contains(f.slug.as_str()),
            "path should contain slug: path={} slug={}",
            f.path,
            f.slug
        );
    }
}
