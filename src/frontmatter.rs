use anyhow::{bail, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::SchemaConfig;

pub const BUILT_IN_TYPES: &[&str] = &[
    "concept",
    "query-result",
    "section",
    "paper",
    "article",
    "documentation",
    "clipping",
    "transcript",
    "note",
    "data",
    "book-chapter",
    "thread",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Claim {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PageFrontmatter {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_when: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_updated: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tldr: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concepts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<Claim>,
}

pub fn parse_frontmatter(content: &str) -> Result<(PageFrontmatter, String)> {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        bail!("no frontmatter block found");
    }
    let after_open = &trimmed[3..];
    let rest = after_open.trim_start_matches('\r').trim_start_matches('\n');
    let close = rest
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("no closing --- found"))?;
    let yaml_str = &rest[..close];
    let after_close = &rest[close + 4..]; // skip \n---
                                          // Strip the line ending after closing ---
    let body = after_close
        .strip_prefix("\r\n")
        .or_else(|| after_close.strip_prefix('\n'))
        .unwrap_or(after_close);

    let fm: PageFrontmatter =
        serde_yaml::from_str(yaml_str).map_err(|e| anyhow::anyhow!("invalid YAML: {e}"))?;
    Ok((fm, body.to_string()))
}

pub fn write_frontmatter(fm: &PageFrontmatter, body: &str) -> String {
    let yaml = serde_yaml::to_string(fm).expect("frontmatter serialization failed");
    format!("---\n{yaml}---\n\n{body}")
}

pub fn generate_minimal_frontmatter(title: &str) -> PageFrontmatter {
    let today = Local::now().format("%Y-%m-%d").to_string();
    PageFrontmatter {
        title: title.to_string(),
        summary: String::new(),
        status: "active".into(),
        last_updated: today,
        r#type: "page".into(),
        tags: Vec::new(),
        read_when: Vec::new(),
        ..Default::default()
    }
}

pub fn scaffold_frontmatter(slug: &str) -> PageFrontmatter {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let last_segment = slug.rsplit('/').next().unwrap_or(slug);
    let title = last_segment
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + c.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    PageFrontmatter {
        title,
        summary: String::new(),
        status: "draft".into(),
        last_updated: today,
        r#type: "page".into(),
        tags: Vec::new(),
        read_when: Vec::new(),
        ..Default::default()
    }
}

pub fn validate_frontmatter(
    fm: &PageFrontmatter,
    schema: &SchemaConfig,
    strictness: &str,
) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    if fm.title.is_empty() {
        bail!("title is required");
    }
    if fm.summary.is_empty() {
        warnings.push("missing required field: summary".to_string());
    }
    if fm.read_when.is_empty() {
        warnings.push("missing required field: read_when".to_string());
    }
    if fm.status.is_empty() {
        warnings.push("missing required field: status".to_string());
    }
    if fm.r#type.is_empty() {
        warnings.push("missing required field: type".to_string());
    }
    if fm.last_updated.is_empty() {
        warnings.push("missing required field: last_updated".to_string());
    }

    if fm.r#type == "source-summary" {
        warnings.push("type 'source-summary' is deprecated — use a specific source type (paper, article, documentation, etc.)".to_string());
    } else if !fm.r#type.is_empty() {
        let known = BUILT_IN_TYPES.contains(&fm.r#type.as_str())
            || fm.r#type == "page"
            || schema.custom_types.iter().any(|t| t == &fm.r#type);
        if !known {
            if strictness == "strict" {
                bail!("unknown type '{}'", fm.r#type);
            } else {
                warnings.push(format!("unknown type '{}'", fm.r#type));
            }
        }
    }

    Ok(warnings)
}

/// Extract title from body: first H1 heading, or fall back to filename stem.
pub fn title_from_body_or_filename(body: &str, filename: &str) -> String {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            let title = heading.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    filename
        .trim_end_matches(".md")
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + c.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
