use llm_wiki::frontmatter::*;
use llm_wiki::slug::Slug;

// ── parse ─────────────────────────────────────────────────────────────────────

#[test]
fn parse_extracts_frontmatter_and_body() {
    let content = "---\ntitle: \"Test Page\"\ntype: concept\nstatus: active\ntags:\n  - test\n  - demo\nsources:\n  - sources/foo\n---\n\n## Body\n\nHello world.\n";

    let page = parse(content);
    assert_eq!(page.title(), Some("Test Page"));
    assert_eq!(page.page_type(), Some("concept"));
    assert_eq!(page.status(), Some("active"));
    assert_eq!(page.tags(), vec!["test", "demo"]);
    assert_eq!(page.string_list("sources"), vec!["sources/foo"]);
    assert!(page.body.contains("## Body"));
    assert!(page.body.contains("Hello world."));
}

#[test]
fn parse_handles_bom() {
    let content = "\u{feff}---\ntitle: \"BOM Page\"\n---\n\nBody.\n";
    let page = parse(content);
    assert_eq!(page.title(), Some("BOM Page"));
}

#[test]
fn parse_no_frontmatter_returns_empty_fm_and_full_body() {
    let content = "# Just a heading\n\nSome body text.\n";
    let page = parse(content);
    assert!(page.frontmatter.is_empty());
    assert!(page.body.contains("# Just a heading"));
}

#[test]
fn parse_no_closing_returns_empty_fm() {
    let content = "---\ntitle: \"Broken\"\nno closing marker\n";
    let page = parse(content);
    assert!(page.frontmatter.is_empty());
}

#[test]
fn parse_strict_errors_on_no_frontmatter() {
    let content = "# Just a heading\n";
    assert!(parse_strict(content).is_err());
}

#[test]
fn parse_strict_errors_on_invalid_yaml() {
    let content = "---\ntitle: [invalid yaml\n  broken: {{\n---\n\nBody\n";
    assert!(parse_strict(content).is_err());
}

#[test]
fn parse_strict_succeeds_on_valid() {
    let content = "---\ntitle: \"OK\"\ntype: page\n---\n\nBody.\n";
    let page = parse_strict(content).unwrap();
    assert_eq!(page.title(), Some("OK"));
}

// ── ParsedPage accessors ─────────────────────────────────────────────────────

#[test]
fn superseded_by_accessor() {
    let content = "---\ntitle: \"Old\"\nsuperseded_by: concepts/new\n---\n\n";
    let page = parse(content);
    assert_eq!(page.superseded_by(), Some("concepts/new"));
}

#[test]
fn superseded_by_absent() {
    let content = "---\ntitle: \"Current\"\n---\n\n";
    let page = parse(content);
    assert_eq!(page.superseded_by(), None);
}

#[test]
fn string_list_missing_key() {
    let content = "---\ntitle: \"Page\"\n---\n\n";
    let page = parse(content);
    assert!(page.string_list("sources").is_empty());
}

// ── write round-trip ──────────────────────────────────────────────────────────

#[test]
fn write_round_trips() {
    let content = "---\ntitle: \"Round Trip\"\ntype: concept\n---\n\n## Body\n\nContent.\n";
    let page = parse(content);
    let output = write(&page.frontmatter, &page.body);
    let page2 = parse(&output);
    assert_eq!(page2.title(), Some("Round Trip"));
    assert_eq!(page2.page_type(), Some("concept"));
    assert!(page2.body.contains("## Body"));
}

#[test]
fn write_produces_valid_structure() {
    let content = "---\ntitle: \"Test\"\n---\n\nBody.\n";
    let page = parse(content);
    let output = write(&page.frontmatter, &page.body);
    assert!(output.starts_with("---\n"));
    assert!(output.contains("\n---\n\n"));
}

// ── generate_minimal ──────────────────────────────────────────────────────────

#[test]
fn generate_minimal_sets_defaults() {
    let fm = generate_minimal("My Title");
    assert_eq!(fm.get("title").unwrap().as_str(), Some("My Title"));
    assert_eq!(fm.get("type").unwrap().as_str(), Some("page"));
    assert_eq!(fm.get("status").unwrap().as_str(), Some("active"));
    assert!(fm.get("last_updated").unwrap().as_str().is_some());
}

// ── scaffold ──────────────────────────────────────────────────────────────────

#[test]
fn scaffold_page() {
    let slug = Slug::try_from("concepts/mixture-of-experts").unwrap();
    let fm = scaffold(&slug, false);
    assert_eq!(
        fm.get("title").unwrap().as_str(),
        Some("Mixture Of Experts")
    );
    assert_eq!(fm.get("type").unwrap().as_str(), Some("page"));
    assert_eq!(fm.get("status").unwrap().as_str(), Some("draft"));
}

#[test]
fn scaffold_section() {
    let slug = Slug::try_from("concepts/scaling").unwrap();
    let fm = scaffold(&slug, true);
    assert_eq!(fm.get("type").unwrap().as_str(), Some("section"));
}

#[test]
fn scaffold_emits_confidence_half() {
    let slug = Slug::try_from("concepts/test").unwrap();
    let fm = scaffold(&slug, false);
    let c = fm.get("confidence").unwrap().as_f64().unwrap();
    assert!((c - 0.5).abs() < f64::EPSILON);
}

// ── confidence getter ─────────────────────────────────────────────────────────

#[test]
fn confidence_maps_legacy_strings() {
    use serde_yaml::Value;
    use std::collections::BTreeMap;

    let mut fm = BTreeMap::new();
    fm.insert("confidence".to_string(), Value::String("high".to_string()));
    assert!((confidence(&fm) - 0.9).abs() < f32::EPSILON);

    fm.insert(
        "confidence".to_string(),
        Value::String("medium".to_string()),
    );
    assert!((confidence(&fm) - 0.5).abs() < f32::EPSILON);

    fm.insert("confidence".to_string(), Value::String("low".to_string()));
    assert!((confidence(&fm) - 0.2).abs() < f32::EPSILON);
}

#[test]
fn confidence_absent_returns_default() {
    use std::collections::BTreeMap;
    let fm = BTreeMap::new();
    assert!((confidence(&fm) - 0.5).abs() < f32::EPSILON);
}

#[test]
fn confidence_numeric_value() {
    use serde_yaml::Value;
    use std::collections::BTreeMap;

    let mut fm = BTreeMap::new();
    fm.insert(
        "confidence".to_string(),
        Value::Number(serde_yaml::Number::from(0.8f64)),
    );
    assert!((confidence(&fm) - 0.8).abs() < 1e-6);
}

#[test]
fn confidence_out_of_range_clamped() {
    use serde_yaml::Value;
    use std::collections::BTreeMap;

    let mut fm = BTreeMap::new();
    fm.insert(
        "confidence".to_string(),
        Value::Number(serde_yaml::Number::from(1.5f64)),
    );
    assert!((confidence(&fm) - 1.0).abs() < f32::EPSILON);

    fm.insert(
        "confidence".to_string(),
        Value::Number(serde_yaml::Number::from(-0.5f64)),
    );
    assert!((confidence(&fm) - 0.0).abs() < f32::EPSILON);
}

// ── title_from_body_or_filename ───────────────────────────────────────────────

#[test]
fn title_from_h1() {
    let title = title_from_body_or_filename("# My Great Title\n\nContent.\n", "fallback.md");
    assert_eq!(title, "My Great Title");
}

#[test]
fn title_from_filename_fallback() {
    let title = title_from_body_or_filename("No heading here.\n", "my-page-name.md");
    assert_eq!(title, "My Page Name");
}

// ── preserves arbitrary fields ────────────────────────────────────────────────

#[test]
fn preserves_unknown_fields() {
    let content = "---\ntitle: \"Skill\"\nname: ingest\ndescription: \"Process sources\"\nallowed-tools: Read Write\n---\n\nBody.\n";
    let page = parse(content);
    assert_eq!(
        page.frontmatter.get("name").unwrap().as_str(),
        Some("ingest")
    );
    assert_eq!(
        page.frontmatter.get("description").unwrap().as_str(),
        Some("Process sources")
    );
    assert_eq!(
        page.frontmatter.get("allowed-tools").unwrap().as_str(),
        Some("Read Write")
    );
}
