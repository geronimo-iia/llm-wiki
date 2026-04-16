use llm_wiki::frontmatter::*;

#[test]
fn parse_frontmatter_round_trips_all_required_fields() {
    let content = "---\ntitle: \"Test Page\"\nsummary: \"A test summary\"\nread_when:\n  - \"When testing\"\nstatus: active\nlast_updated: \"2025-07-15\"\ntype: concept\ntags:\n  - test\nsources:\n  - sources/foo\nconcepts:\n  - concepts/bar\nconfidence: high\nclaims:\n  - text: \"A claim\"\n    confidence: high\n    section: \"Results\"\n---\n\n## Body\n\nHello world.\n";

    let (fm, body) = parse_frontmatter(content).unwrap();
    assert_eq!(fm.title, "Test Page");
    assert_eq!(fm.summary, "A test summary");
    assert_eq!(fm.read_when, vec!["When testing"]);
    assert_eq!(fm.status, "active");
    assert_eq!(fm.last_updated, "2025-07-15");
    assert_eq!(fm.r#type, "concept");
    assert_eq!(fm.tags, vec!["test"]);
    assert_eq!(fm.sources, vec!["sources/foo"]);
    assert_eq!(fm.concepts, vec!["concepts/bar"]);
    assert_eq!(fm.confidence, Some("high".into()));
    assert_eq!(fm.claims.len(), 1);
    assert_eq!(fm.claims[0].text, "A claim");
    assert!(body.contains("## Body"));
    assert!(body.contains("Hello world."));

    // Round-trip
    let output = write_frontmatter(&fm, &body);
    let (fm2, body2) = parse_frontmatter(&output).unwrap();
    assert_eq!(fm, fm2);
    assert_eq!(body.trim(), body2.trim());
}

#[test]
fn parse_frontmatter_returns_error_on_invalid_yaml() {
    let content = "---\ntitle: [invalid yaml\n  broken: {{\n---\n\nBody\n";
    let result = parse_frontmatter(content);
    assert!(result.is_err());
}

#[test]
fn parse_frontmatter_returns_error_when_no_frontmatter_block() {
    let content = "# Just a heading\n\nSome body text.\n";
    let result = parse_frontmatter(content);
    assert!(result.is_err());
}

#[test]
fn write_frontmatter_produces_valid_yaml_block_blank_line_body() {
    let fm = PageFrontmatter {
        title: "My Page".into(),
        summary: "A summary".into(),
        status: "active".into(),
        last_updated: "2025-07-15".into(),
        r#type: "concept".into(),
        ..Default::default()
    };
    let body = "## Overview\n\nContent here.\n";
    let output = write_frontmatter(&fm, body);

    assert!(output.starts_with("---\n"));
    assert!(output.contains("\n---\n\n"));
    assert!(output.ends_with("Content here.\n"));

    // Verify it parses back
    let (fm2, body2) = parse_frontmatter(&output).unwrap();
    assert_eq!(fm2.title, "My Page");
    assert_eq!(body2.trim(), body.trim());
}

#[test]
fn generate_minimal_frontmatter_sets_title_from_h1_falls_back_to_filename() {
    let body_with_h1 = "# My Great Title\n\nSome content.\n";
    let title = title_from_body_or_filename(body_with_h1, "fallback-name.md");
    assert_eq!(title, "My Great Title");

    let body_no_h1 = "Some content without heading.\n";
    let title = title_from_body_or_filename(body_no_h1, "my-page-name.md");
    assert_eq!(title, "My Page Name");
}

#[test]
fn generate_minimal_frontmatter_sets_status_active_type_page() {
    let fm = generate_minimal_frontmatter("Test");
    assert_eq!(fm.status, "active");
    assert_eq!(fm.r#type, "page");
    assert!(!fm.last_updated.is_empty());
}

#[test]
fn scaffold_frontmatter_derives_title_from_slug_segments() {
    let fm = scaffold_frontmatter("concepts/mixture-of-experts");
    assert_eq!(fm.title, "Mixture Of Experts");
}

#[test]
fn scaffold_frontmatter_sets_status_draft_type_page() {
    let fm = scaffold_frontmatter("concepts/foo");
    assert_eq!(fm.status, "draft");
    assert_eq!(fm.r#type, "page");
    assert!(!fm.last_updated.is_empty());
}

// ── Phase 3 — validation tests ───────────────────────────────────────────────

use llm_wiki::config::SchemaConfig;

fn valid_fm() -> PageFrontmatter {
    PageFrontmatter {
        title: "Test".into(),
        summary: "A summary".into(),
        read_when: vec!["When testing".into()],
        status: "active".into(),
        last_updated: "2025-07-15".into(),
        r#type: "concept".into(),
        ..Default::default()
    }
}

#[test]
fn validate_frontmatter_passes_for_fully_valid_page() {
    let fm = valid_fm();
    let schema = SchemaConfig::default();
    let warnings = validate_frontmatter(&fm, &schema, "strict").unwrap();
    assert!(warnings.is_empty());
}

#[test]
fn validate_frontmatter_warns_on_missing_read_when() {
    let mut fm = valid_fm();
    fm.read_when = vec![];
    let schema = SchemaConfig::default();
    let warnings = validate_frontmatter(&fm, &schema, "loose").unwrap();
    assert!(warnings.iter().any(|w| w.contains("read_when")));
}

#[test]
fn validate_frontmatter_warns_on_missing_summary() {
    let mut fm = valid_fm();
    fm.summary = String::new();
    let schema = SchemaConfig::default();
    let warnings = validate_frontmatter(&fm, &schema, "loose").unwrap();
    assert!(warnings.iter().any(|w| w.contains("summary")));
}

#[test]
fn validate_frontmatter_warns_on_source_summary_type() {
    let mut fm = valid_fm();
    fm.r#type = "source-summary".into();
    let schema = SchemaConfig::default();
    let warnings = validate_frontmatter(&fm, &schema, "loose").unwrap();
    assert!(warnings
        .iter()
        .any(|w| w.contains("source-summary") && w.contains("deprecated")));
}

#[test]
fn validate_frontmatter_loose_warns_on_unknown_type() {
    let mut fm = valid_fm();
    fm.r#type = "alien-type".into();
    let schema = SchemaConfig::default();
    let warnings = validate_frontmatter(&fm, &schema, "loose").unwrap();
    assert!(warnings.iter().any(|w| w.contains("unknown type")));
}

#[test]
fn validate_frontmatter_strict_errors_on_unknown_type() {
    let mut fm = valid_fm();
    fm.r#type = "alien-type".into();
    let schema = SchemaConfig::default();
    let result = validate_frontmatter(&fm, &schema, "strict");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown type"));
}

#[test]
fn validate_frontmatter_accepts_custom_type_from_schema() {
    let mut fm = valid_fm();
    fm.r#type = "patent".into();
    let schema = SchemaConfig {
        custom_types: vec!["patent".into()],
    };
    let warnings = validate_frontmatter(&fm, &schema, "strict").unwrap();
    assert!(!warnings.iter().any(|w| w.contains("unknown type")));
}
