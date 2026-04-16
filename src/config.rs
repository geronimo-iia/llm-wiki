use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ── Section structs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalSection {
    #[serde(default)]
    pub default_wiki: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEntry {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_search_top_k")]
    pub search_top_k: u32,
    #[serde(default = "default_true")]
    pub search_excerpt: bool,
    #[serde(default)]
    pub search_sections: bool,
    #[serde(default = "default_page_mode")]
    pub page_mode: String,
    #[serde(default = "default_list_page_size")]
    pub list_page_size: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            search_top_k: 10,
            search_excerpt: true,
            search_sections: false,
            page_mode: "flat".into(),
            list_page_size: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReadConfig {
    #[serde(default)]
    pub no_frontmatter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexConfig {
    #[serde(default)]
    pub auto_rebuild: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    #[serde(default = "default_graph_format")]
    pub format: String,
    #[serde(default = "default_graph_depth")]
    pub depth: u32,
    #[serde(default)]
    pub r#type: Vec<String>,
    #[serde(default)]
    pub output: String,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            format: "mermaid".into(),
            depth: 3,
            r#type: Vec::new(),
            output: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeConfig {
    #[serde(default)]
    pub sse: bool,
    #[serde(default = "default_sse_port")]
    pub sse_port: u16,
    #[serde(default)]
    pub acp: bool,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            sse: false,
            sse_port: 8080,
            acp: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    #[serde(default = "default_true")]
    pub fix_missing_stubs: bool,
    #[serde(default = "default_true")]
    pub fix_empty_sections: bool,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            fix_missing_stubs: true,
            fix_empty_sections: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    #[serde(default = "default_type_strictness")]
    pub type_strictness: String,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            type_strictness: "loose".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemaConfig {
    #[serde(default)]
    pub custom_types: Vec<String>,
}

// ── Composite configs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub global: GlobalSection,
    #[serde(default)]
    pub wikis: Vec<WikiEntry>,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub read: ReadConfig,
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub graph: GraphConfig,
    #[serde(default)]
    pub serve: ServeConfig,
    #[serde(default)]
    pub validation: ValidationConfig,
    #[serde(default)]
    pub lint: LintConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WikiConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub defaults: Option<Defaults>,
    #[serde(default)]
    pub validation: Option<ValidationConfig>,
    #[serde(default)]
    pub lint: Option<LintConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedConfig {
    pub defaults: Defaults,
    pub read: ReadConfig,
    pub index: IndexConfig,
    pub graph: GraphConfig,
    pub serve: ServeConfig,
    pub validation: ValidationConfig,
    pub lint: LintConfig,
}

// ── Default value helpers ─────────────────────────────────────────────────────

fn default_search_top_k() -> u32 {
    10
}
fn default_true() -> bool {
    true
}
fn default_page_mode() -> String {
    "flat".into()
}
fn default_list_page_size() -> u32 {
    20
}
fn default_graph_format() -> String {
    "mermaid".into()
}
fn default_graph_depth() -> u32 {
    3
}
fn default_sse_port() -> u16 {
    8080
}
fn default_type_strictness() -> String {
    "loose".into()
}

// ── Functions ─────────────────────────────────────────────────────────────────

pub fn resolve(global: &GlobalConfig, per_wiki: &WikiConfig) -> ResolvedConfig {
    let defaults = if let Some(pw) = &per_wiki.defaults {
        Defaults {
            search_top_k: pw.search_top_k,
            search_excerpt: pw.search_excerpt,
            search_sections: pw.search_sections,
            page_mode: pw.page_mode.clone(),
            list_page_size: pw.list_page_size,
        }
    } else {
        global.defaults.clone()
    };

    let validation = if let Some(pw) = &per_wiki.validation {
        pw.clone()
    } else {
        global.validation.clone()
    };

    let lint = if let Some(pw) = &per_wiki.lint {
        pw.clone()
    } else {
        global.lint.clone()
    };

    ResolvedConfig {
        defaults,
        read: global.read.clone(),
        index: global.index.clone(),
        graph: global.graph.clone(),
        serve: global.serve.clone(),
        validation,
        lint,
    }
}

pub fn load_global(path: &Path) -> Result<GlobalConfig> {
    if !path.exists() {
        return Ok(GlobalConfig::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let config: GlobalConfig =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config)
}

pub fn load_wiki(wiki_root: &Path) -> Result<WikiConfig> {
    let path = wiki_root.join("wiki.toml");
    if !path.exists() {
        return Ok(WikiConfig::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let config: WikiConfig =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config)
}

pub fn load_schema(wiki_root: &Path) -> Result<SchemaConfig> {
    let path = wiki_root.join("schema.md");
    if !path.exists() {
        return Ok(SchemaConfig::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let custom_types = content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("- type:") || trimmed.starts_with("- `type:") {
                let after = trimmed
                    .trim_start_matches("- type:")
                    .trim_start_matches("- `type:")
                    .trim()
                    .trim_end_matches('`')
                    .trim()
                    .to_string();
                if after.is_empty() {
                    None
                } else {
                    Some(after)
                }
            } else {
                None
            }
        })
        .collect();

    Ok(SchemaConfig { custom_types })
}

/// Save a GlobalConfig back to disk.
pub fn save_global(config: &GlobalConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}
