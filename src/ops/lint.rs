use std::collections::HashSet;

use anyhow::Result;
use serde::Serialize;
use tantivy::schema::Value;
use tantivy::{
    Term,
    query::{AllQuery, TermQuery},
    schema::IndexRecordOption,
};

use crate::engine::EngineState;
use crate::index_schema::IndexSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LintFinding {
    pub slug: String,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    pub wiki: String,
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub findings: Vec<LintFinding>,
}

pub fn run_lint(
    engine: &EngineState,
    wiki_name: &str,
    rules: Option<&str>,
    severity_filter: Option<&str>,
) -> Result<LintReport> {
    let active_rules: HashSet<&str> = match rules {
        None | Some("") => [
            "orphan",
            "broken-link",
            "missing-fields",
            "stale",
            "unknown-type",
        ]
        .iter()
        .copied()
        .collect(),
        Some(s) => s.split(',').map(str::trim).collect(),
    };

    let space = engine.space(wiki_name)?;
    let searcher = space.index_manager.searcher()?;
    let is = &space.index_schema;
    let resolved = space.resolved_config(&engine.config);
    let lint_cfg = &resolved.lint;

    let mut findings: Vec<LintFinding> = Vec::new();

    if active_rules.contains("orphan") {
        findings.extend(rule_orphan(&searcher, is)?);
    }
    if active_rules.contains("broken-link") {
        findings.extend(rule_broken_link(&searcher, is)?);
    }
    if active_rules.contains("missing-fields") {
        findings.extend(rule_missing_fields(&searcher, is, &space.type_registry)?);
    }
    if active_rules.contains("stale") {
        findings.extend(rule_stale(
            &searcher,
            is,
            lint_cfg.stale_days,
            lint_cfg.stale_confidence_threshold,
        )?);
    }
    if active_rules.contains("unknown-type") {
        findings.extend(rule_unknown_type(&searcher, is, &space.type_registry)?);
    }

    // Apply severity filter
    if let Some(sev) = severity_filter {
        let sev = sev.trim().to_lowercase();
        findings.retain(|f| f.severity.to_string() == sev);
    }

    findings.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.rule.cmp(b.rule)));

    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    let total = findings.len();

    Ok(LintReport {
        wiki: wiki_name.to_string(),
        total,
        errors,
        warnings,
        findings,
    })
}

// ── Rule: orphan ──────────────────────────────────────────────────────────────

fn rule_orphan(searcher: &tantivy::Searcher, is: &IndexSchema) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_type = is.field("type");

    // Collect all slugs referenced in body_links across all docs
    let mut all_linked: HashSet<String> = HashSet::new();
    let f_body_links = is.field("body_links");

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        for val in doc.get_all(f_body_links) {
            if let Some(s) = val.as_str() {
                all_linked.insert(s.to_string());
            }
        }
        // Also count frontmatter edge fields as incoming-link evidence
        for field_name in &["sources", "concepts", "document_refs", "superseded_by"] {
            if let Some(f) = is.try_field(field_name) {
                for val in doc.get_all(f) {
                    if let Some(s) = val.as_str() {
                        all_linked.insert(s.to_string());
                    }
                }
            }
        }
    }

    let mut findings = Vec::new();
    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Sections are structural — not flagged as orphans
        if page_type == "section" {
            continue;
        }
        // Root/index pages are exempt
        if slug == "index" || slug.ends_with("/index") {
            continue;
        }

        if !all_linked.contains(&slug) {
            findings.push(LintFinding {
                slug,
                rule: "orphan",
                severity: Severity::Warning,
                message: "no incoming links".to_string(),
            });
        }
    }

    Ok(findings)
}

// ── Rule: broken-link ─────────────────────────────────────────────────────────

fn slug_exists(searcher: &tantivy::Searcher, is: &IndexSchema, slug: &str) -> Result<bool> {
    let f_slug = is.field("slug");
    let term = Term::from_field_text(f_slug, slug);
    let query = TermQuery::new(term, IndexRecordOption::Basic);
    let results = searcher.search(&query, &tantivy::collector::DocSetCollector)?;
    Ok(!results.is_empty())
}

fn rule_broken_link(searcher: &tantivy::Searcher, is: &IndexSchema) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let link_fields = [
        "body_links",
        "sources",
        "concepts",
        "document_refs",
        "superseded_by",
    ];

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }

        for field_name in &link_fields {
            let f = match is.try_field(field_name) {
                Some(f) => f,
                None => continue,
            };
            for val in doc.get_all(f) {
                let target = match val.as_str() {
                    Some(s) => s,
                    None => continue,
                };
                // Skip cross-wiki links (imp-10's concern)
                if target.starts_with("wiki://") {
                    continue;
                }
                if !slug_exists(searcher, is, target)? {
                    findings.push(LintFinding {
                        slug: slug.clone(),
                        rule: "broken-link",
                        severity: Severity::Error,
                        message: format!("broken link in {field_name}: {target}"),
                    });
                }
            }
        }
    }

    Ok(findings)
}

// ── Rule: missing-fields ──────────────────────────────────────────────────────

fn rule_missing_fields(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    registry: &crate::type_registry::SpaceTypeRegistry,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_type = is.field("type");

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if page_type.is_empty() || !registry.is_known(&page_type) {
            continue;
        }

        // Get required fields from JSON schema
        let required = registry.required_fields(&page_type);
        for field_name in &required {
            // Check via index field presence
            let present = if let Some(f) = is.try_field(field_name) {
                doc.get_first(f).is_some()
            } else {
                // Field not in index schema — can't check, skip
                true
            };
            if !present {
                findings.push(LintFinding {
                    slug: slug.clone(),
                    rule: "missing-fields",
                    severity: Severity::Error,
                    message: format!("required field missing: {field_name}"),
                });
            }
        }
    }

    Ok(findings)
}

// ── Rule: stale ───────────────────────────────────────────────────────────────

fn rule_stale(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    stale_days: u32,
    stale_confidence_threshold: f32,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_last_updated = match is.try_field("last_updated") {
        Some(f) => f,
        None => return Ok(vec![]),
    };
    let f_confidence = is.try_field("confidence");

    let today = chrono::Utc::now().date_naive();
    let threshold_date = today - chrono::Duration::days(stale_days as i64);

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }

        let date_str = doc
            .get_first(f_last_updated)
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let is_old = if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            date < threshold_date
        } else {
            // No valid date — treat as old
            true
        };

        if !is_old {
            continue;
        }

        // Check confidence if the field is indexed
        let is_low_confidence = if let Some(f_conf) = f_confidence {
            match doc.get_first(f_conf).and_then(|v| v.as_f64()) {
                Some(v) => (v as f32) < stale_confidence_threshold,
                None => true, // No confidence value — treat as low
            }
        } else {
            // Field not indexed — fall back to date-only
            true
        };

        if is_old && is_low_confidence {
            let age_note = if date_str.is_empty() {
                "no last_updated date".to_string()
            } else {
                format!("last updated {date_str}")
            };
            findings.push(LintFinding {
                slug,
                rule: "stale",
                severity: Severity::Warning,
                message: format!("stale page: {age_note}"),
            });
        }
    }

    Ok(findings)
}

// ── Rule: unknown-type ────────────────────────────────────────────────────────

fn rule_unknown_type(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    registry: &crate::type_registry::SpaceTypeRegistry,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_type = is.field("type");

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let page_type = doc.get_first(f_type).and_then(|v| v.as_str()).unwrap_or("");
        if page_type.is_empty() {
            continue;
        }
        if !registry.is_known(page_type) {
            findings.push(LintFinding {
                slug,
                rule: "unknown-type",
                severity: Severity::Error,
                message: format!("unknown type: {page_type}"),
            });
        }
    }

    Ok(findings)
}
