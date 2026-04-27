//! Opt-in secret redaction for page bodies. Enabled per-ingest via `redact: true`.
//! Built-in patterns cover common API keys, tokens, and emails. Custom patterns
//! are added via `[redact.patterns]` in config. Redaction is lossy by design.

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::config::RedactConfig;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionMatch {
    pub pattern_name: String,
    pub line_number: usize,
}

/// Report of all redaction substitutions applied to a single page body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionReport {
    pub slug: String,
    pub matches: Vec<RedactionMatch>,
}

struct RedactPattern {
    name: &'static str,
    regex: Regex,
    replacement: &'static str,
}

// ── Built-in patterns ─────────────────────────────────────────────────────────

fn builtin_patterns() -> Vec<RedactPattern> {
    let specs: &[(&'static str, &'static str, &'static str)] = &[
        (
            "github-pat",
            r"ghp_[A-Za-z0-9]{36}",
            "[REDACTED:github-pat]",
        ),
        ("openai-key", r"sk-[A-Za-z0-9]{48}", "[REDACTED:openai-key]"),
        (
            "anthropic-key",
            r"sk-ant-[A-Za-z0-9\-]{90,}",
            "[REDACTED:anthropic-key]",
        ),
        (
            "aws-access-key",
            r"AKIA[0-9A-Z]{16}",
            "[REDACTED:aws-access-key]",
        ),
        (
            "bearer-token",
            r"Bearer [A-Za-z0-9\-._~+/]{20,}",
            "[REDACTED:bearer-token]",
        ),
        (
            "email",
            r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}",
            "[REDACTED:email]",
        ),
    ];
    specs
        .iter()
        .map(|(name, pat, rep)| RedactPattern {
            name,
            regex: Regex::new(pat).expect("builtin regex is valid"),
            replacement: rep,
        })
        .collect()
}

// ── Pattern builder ───────────────────────────────────────────────────────────

struct CompiledPattern {
    name: String,
    regex: Regex,
    replacement: String,
}

fn build_patterns(config: &RedactConfig) -> Vec<CompiledPattern> {
    let disabled: std::collections::HashSet<&str> =
        config.disable.iter().map(String::as_str).collect();

    let mut patterns: Vec<CompiledPattern> = builtin_patterns()
        .into_iter()
        .filter(|p| !disabled.contains(p.name))
        .map(|p| CompiledPattern {
            name: p.name.to_string(),
            regex: p.regex,
            replacement: p.replacement.to_string(),
        })
        .collect();

    for custom in &config.patterns {
        match Regex::new(&custom.pattern) {
            Ok(re) => patterns.push(CompiledPattern {
                name: custom.name.clone(),
                regex: re,
                replacement: custom.replacement.clone(),
            }),
            Err(e) => {
                tracing::warn!(
                    pattern = %custom.name,
                    error = %e,
                    "skipping invalid custom redaction pattern"
                );
            }
        }
    }

    patterns
}

// ── Core redaction ────────────────────────────────────────────────────────────

/// Redact secrets from `body` (never frontmatter). Returns the redacted body
/// and a list of matches (pattern name + 1-based line number). Lossy by design.
pub fn redact_body(body: &str, config: &RedactConfig) -> (String, Vec<RedactionMatch>) {
    let patterns = build_patterns(config);
    let mut matches: Vec<RedactionMatch> = Vec::new();
    let mut result = String::with_capacity(body.len());

    for (line_idx, line) in body.lines().enumerate() {
        let line_number = line_idx + 1;
        let mut current = line.to_string();
        for pat in &patterns {
            if pat.regex.is_match(&current) {
                matches.push(RedactionMatch {
                    pattern_name: pat.name.clone(),
                    line_number,
                });
                current = pat
                    .regex
                    .replace_all(&current, pat.replacement.as_str())
                    .into_owned();
            }
        }
        result.push_str(&current);
        result.push('\n');
    }

    // Preserve original trailing newline behaviour
    if !body.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    (result, matches)
}
