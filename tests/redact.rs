use llm_wiki::config::RedactConfig;
use llm_wiki::ops::redact::redact_body;

fn cfg_default() -> RedactConfig {
    RedactConfig::default()
}

fn cfg_disable(names: &[&str]) -> RedactConfig {
    RedactConfig {
        disable: names.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

fn cfg_custom(name: &str, pattern: &str, replacement: &str) -> RedactConfig {
    use llm_wiki::config::CustomPattern;
    RedactConfig {
        patterns: vec![CustomPattern {
            name: name.to_string(),
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
        }],
        ..Default::default()
    }
}

// ── Built-in pattern tests ────────────────────────────────────────────────────

#[test]
fn github_pat_is_redacted() {
    let body = "token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890\n";
    let (out, matches) = redact_body(body, &cfg_default());
    assert!(out.contains("[REDACTED:github-pat]"), "body: {out}");
    assert!(!out.contains("ghp_"), "original value must be gone");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].pattern_name, "github-pat");
    assert_eq!(matches[0].line_number, 1);
}

#[test]
fn openai_key_is_redacted() {
    let key = "sk-".to_string() + &"A".repeat(48);
    let body = format!("key: {key}\n");
    let (out, matches) = redact_body(&body, &cfg_default());
    assert!(out.contains("[REDACTED:openai-key]"), "body: {out}");
    assert_eq!(matches[0].pattern_name, "openai-key");
}

#[test]
fn anthropic_key_is_redacted() {
    let key = "sk-ant-".to_string() + &"a".repeat(90);
    let body = format!("key: {key}\n");
    let (out, matches) = redact_body(&body, &cfg_default());
    assert!(out.contains("[REDACTED:anthropic-key]"), "body: {out}");
    assert_eq!(matches[0].pattern_name, "anthropic-key");
}

#[test]
fn aws_access_key_is_redacted() {
    let body = "access_key: AKIAIOSFODNN7EXAMPLE\n";
    let (out, matches) = redact_body(body, &cfg_default());
    assert!(out.contains("[REDACTED:aws-access-key]"), "body: {out}");
    assert_eq!(matches[0].pattern_name, "aws-access-key");
}

#[test]
fn bearer_token_is_redacted() {
    let body = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\n";
    let (out, matches) = redact_body(body, &cfg_default());
    assert!(out.contains("[REDACTED:bearer-token]"), "body: {out}");
    assert_eq!(matches[0].pattern_name, "bearer-token");
}

#[test]
fn email_is_redacted() {
    let body = "Contact: alice@example.com for help.\n";
    let (out, matches) = redact_body(body, &cfg_default());
    assert!(out.contains("[REDACTED:email]"), "body: {out}");
    assert!(!out.contains("alice@example.com"));
    assert_eq!(matches[0].pattern_name, "email");
}

// ── disable built-in ──────────────────────────────────────────────────────────

#[test]
fn disable_email_skips_email_pattern() {
    let body = "Contact: bob@example.com\n";
    let cfg = cfg_disable(&["email"]);
    let (out, matches) = redact_body(body, &cfg);
    assert!(
        out.contains("bob@example.com"),
        "email should not be redacted"
    );
    assert!(matches.is_empty());
}

#[test]
fn disable_one_pattern_leaves_others_active() {
    let body = "email: user@test.com\naccess_key: AKIAIOSFODNN7EXAMPLE\n";
    let cfg = cfg_disable(&["email"]);
    let (out, _) = redact_body(body, &cfg);
    assert!(
        out.contains("user@test.com"),
        "email should not be redacted"
    );
    assert!(
        out.contains("[REDACTED:aws-access-key]"),
        "aws key should still be redacted"
    );
}

// ── custom pattern ────────────────────────────────────────────────────────────

#[test]
fn custom_pattern_is_applied() {
    let body = "employee: EMP-123456 is the assignee\n";
    let cfg = cfg_custom("employee-id", r"EMP-[0-9]{6}", "[REDACTED:employee-id]");
    let (out, matches) = redact_body(body, &cfg);
    assert!(out.contains("[REDACTED:employee-id]"), "body: {out}");
    assert!(!out.contains("EMP-123456"));
    assert_eq!(matches[0].pattern_name, "employee-id");
}

#[test]
fn custom_pattern_runs_alongside_builtins() {
    let body = "employee: EMP-654321\ncontact: alice@company.org\n";
    let cfg = cfg_custom("employee-id", r"EMP-[0-9]{6}", "[REDACTED:employee-id]");
    let (out, matches) = redact_body(body, &cfg);
    assert!(out.contains("[REDACTED:employee-id]"));
    assert!(out.contains("[REDACTED:email]"));
    assert_eq!(matches.len(), 2);
}

// ── no-op when redact is false ────────────────────────────────────────────────

#[test]
fn no_matches_when_body_is_clean() {
    let body = "This is a clean page with no secrets.\n";
    let (out, matches) = redact_body(body, &cfg_default());
    assert_eq!(out, body);
    assert!(matches.is_empty());
}

// ── line numbers ──────────────────────────────────────────────────────────────

#[test]
fn match_reports_correct_line_number() {
    let body = "line one\nline two\nemail: secret@example.com\nline four\n";
    let (_, matches) = redact_body(body, &cfg_default());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line_number, 3);
}

// ── ingest integration ────────────────────────────────────────────────────────

#[test]
fn ingest_redact_false_leaves_body_unchanged() {
    use llm_wiki::config::ValidationConfig;
    use llm_wiki::git;
    use llm_wiki::ingest::{IngestOptions, ingest};
    use llm_wiki::type_registry::SpaceTypeRegistry;
    use std::path::Path;

    let dir = tempfile::tempdir().unwrap();
    let wiki_root = dir.path().join("wiki");
    std::fs::create_dir_all(&wiki_root).unwrap();
    git::init_repo(dir.path()).unwrap();
    std::fs::write(dir.path().join("README.md"), "# test\n").unwrap();
    git::commit(dir.path(), "init").unwrap();

    let secret = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
    let content =
        format!("---\ntitle: \"Test\"\ntype: concept\nstatus: active\n---\n\nSecret: {secret}\n");
    std::fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    std::fs::write(wiki_root.join("concepts/test.md"), &content).unwrap();

    let opts = IngestOptions {
        redact: None, // no redaction
        ..Default::default()
    };
    ingest(
        Path::new("concepts/test.md"),
        &opts,
        &wiki_root,
        &SpaceTypeRegistry::from_embedded(),
        &ValidationConfig::default(),
    )
    .unwrap();

    let after = std::fs::read_to_string(wiki_root.join("concepts/test.md")).unwrap();
    assert!(
        after.contains(secret),
        "file should be unchanged without redact"
    );
}
