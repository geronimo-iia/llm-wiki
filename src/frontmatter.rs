use std::collections::BTreeMap;

use anyhow::{Result, bail};
use chrono::Local;
use serde_yaml::Value;

/// Read page-level `confidence` from frontmatter; map legacy string values.
pub fn confidence(fm: &BTreeMap<String, Value>) -> f32 {
    match fm.get("confidence") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.5) as f32,
        Some(Value::String(s)) => match s.as_str() {
            "high" => 0.9,
            "medium" => 0.5,
            "low" => 0.2,
            _ => 0.5,
        },
        _ => 0.5,
    }
    .clamp(0.0, 1.0)
}

use crate::slug::Slug;

/// A parsed markdown page — untyped frontmatter + body.
#[derive(Debug, Clone)]
pub struct ParsedPage {
    /// Parsed YAML frontmatter key-value pairs.
    pub frontmatter: BTreeMap<String, Value>,
    /// Markdown body text after the closing `---` delimiter.
    pub body: String,
}

impl ParsedPage {
    /// Return the `title` frontmatter value, if present.
    pub fn title(&self) -> Option<&str> {
        self.frontmatter.get("title").and_then(|v| v.as_str())
    }

    /// Return the `type` frontmatter value, if present.
    pub fn page_type(&self) -> Option<&str> {
        self.frontmatter.get("type").and_then(|v| v.as_str())
    }

    /// Return the `status` frontmatter value, if present.
    pub fn status(&self) -> Option<&str> {
        self.frontmatter.get("status").and_then(|v| v.as_str())
    }

    /// Return the `tags` list from frontmatter; empty if absent.
    pub fn tags(&self) -> Vec<&str> {
        self.frontmatter
            .get("tags")
            .and_then(|v| v.as_sequence())
            .map(|seq| seq.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default()
    }

    /// Return the `superseded_by` frontmatter value, if present.
    pub fn superseded_by(&self) -> Option<&str> {
        self.frontmatter
            .get("superseded_by")
            .and_then(|v| v.as_str())
    }

    /// Return a YAML sequence field as a `Vec<&str>`; empty if absent or not a sequence.
    pub fn string_list(&self, key: &str) -> Vec<&str> {
        self.frontmatter
            .get(key)
            .and_then(|v| v.as_sequence())
            .map(|seq| seq.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default()
    }
}

/// Parse a markdown file into frontmatter (YAML) and body.
///
/// If no `---` opening is found, returns empty frontmatter and the
/// entire content as body.
pub fn parse(content: &str) -> ParsedPage {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return ParsedPage {
            frontmatter: BTreeMap::new(),
            body: trimmed.to_string(),
        };
    }
    let after_open = &trimmed[3..];
    let rest = after_open.trim_start_matches('\r').trim_start_matches('\n');
    let Some(close) = rest.find("\n---") else {
        return ParsedPage {
            frontmatter: BTreeMap::new(),
            body: trimmed.to_string(),
        };
    };
    let yaml_str = &rest[..close];
    let after_close = &rest[close + 4..];
    let body = after_close
        .strip_prefix("\r\n")
        .or_else(|| after_close.strip_prefix('\n'))
        .unwrap_or(after_close);

    let frontmatter: BTreeMap<String, Value> = serde_yaml::from_str(yaml_str).unwrap_or_default();

    ParsedPage {
        frontmatter,
        body: body.to_string(),
    }
}

/// Parse frontmatter strictly — error if no frontmatter block or invalid YAML.
pub fn parse_strict(content: &str) -> Result<ParsedPage> {
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
    let after_close = &rest[close + 4..];
    let body = after_close
        .strip_prefix("\r\n")
        .or_else(|| after_close.strip_prefix('\n'))
        .unwrap_or(after_close);

    let frontmatter: BTreeMap<String, Value> =
        serde_yaml::from_str(yaml_str).map_err(|e| anyhow::anyhow!("invalid YAML: {e}"))?;

    Ok(ParsedPage {
        frontmatter,
        body: body.to_string(),
    })
}

/// Serialize frontmatter + body back to a markdown string.
pub fn write(frontmatter: &BTreeMap<String, Value>, body: &str) -> String {
    let yaml = serde_yaml::to_string(frontmatter).expect("frontmatter serialization failed");
    format!("---\n{yaml}---\n\n{body}")
}

/// Generate minimal frontmatter for a file without any.
pub fn generate_minimal(title: &str) -> BTreeMap<String, Value> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut fm = BTreeMap::new();
    fm.insert("title".into(), Value::String(title.into()));
    fm.insert("type".into(), Value::String("page".into()));
    fm.insert("status".into(), Value::String("active".into()));
    fm.insert("last_updated".into(), Value::String(today));
    fm
}

/// Scaffold frontmatter for a new page or section.
pub fn scaffold(slug: &Slug, section: bool) -> BTreeMap<String, Value> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut fm = BTreeMap::new();
    fm.insert("title".into(), Value::String(slug.title()));
    fm.insert("status".into(), Value::String("draft".into()));
    fm.insert("last_updated".into(), Value::String(today));
    fm.insert(
        "type".into(),
        Value::String(if section { "section" } else { "page" }.into()),
    );
    fm.insert(
        "confidence".into(),
        Value::Number(serde_yaml::Number::from(0.5f64)),
    );
    fm
}

/// Extract title from body: first `# Heading`, or fall back to filename stem title-cased.
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
    // Fall back to filename stem, title-cased
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
