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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    #[serde(default)]
    pub auto_rebuild: bool,
    #[serde(default = "default_true")]
    pub auto_recovery: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            auto_rebuild: false,
            auto_recovery: true,
        }
    }
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
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    #[serde(default = "default_restart_backoff")]
    pub restart_backoff: u32,
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u32,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            sse: false,
            sse_port: 8080,
            acp: false,
            max_restarts: 10,
            restart_backoff: 1,
            heartbeat_secs: 60,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_path")]
    pub log_path: String,
    #[serde(default = "default_log_rotation")]
    pub log_rotation: String,
    #[serde(default = "default_log_max_files")]
    pub log_max_files: u32,
    #[serde(default = "default_log_format")]
    pub log_format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_path: default_log_path(),
            log_rotation: "daily".into(),
            log_max_files: 7,
            log_format: "text".into(),
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
    #[serde(default)]
    pub logging: LoggingConfig,
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
    pub read: Option<ReadConfig>,
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
fn default_max_restarts() -> u32 {
    10
}
fn default_restart_backoff() -> u32 {
    1
}
fn default_heartbeat_secs() -> u32 {
    60
}
fn default_type_strictness() -> String {
    "loose".into()
}
fn default_log_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home)
        .join(".wiki")
        .join("logs")
        .to_string_lossy()
        .into()
}
fn default_log_rotation() -> String {
    "daily".into()
}
fn default_log_max_files() -> u32 {
    7
}
fn default_log_format() -> String {
    "text".into()
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
        read: if let Some(pw) = &per_wiki.read {
            pw.clone()
        } else {
            global.read.clone()
        },
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

/// Save a WikiConfig to `wiki.toml` inside the wiki root.
pub fn save_wiki(config: &WikiConfig, wiki_root: &Path) -> Result<()> {
    let path = wiki_root.join("wiki.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Set a global config value.
pub fn set_global_config_value(global: &mut GlobalConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "global.default_wiki" => global.global.default_wiki = value.into(),
        "defaults.search_top_k" => global.defaults.search_top_k = value.parse()?,
        "defaults.search_excerpt" => global.defaults.search_excerpt = value.parse()?,
        "defaults.search_sections" => global.defaults.search_sections = value.parse()?,
        "defaults.page_mode" => global.defaults.page_mode = value.into(),
        "defaults.list_page_size" => global.defaults.list_page_size = value.parse()?,
        "read.no_frontmatter" => global.read.no_frontmatter = value.parse()?,
        "index.auto_rebuild" => global.index.auto_rebuild = value.parse()?,
        "index.auto_recovery" => global.index.auto_recovery = value.parse()?,
        "graph.format" => global.graph.format = value.into(),
        "graph.depth" => global.graph.depth = value.parse()?,
        "graph.output" => global.graph.output = value.into(),
        "serve.sse" => global.serve.sse = value.parse()?,
        "serve.sse_port" => global.serve.sse_port = value.parse()?,
        "serve.acp" => global.serve.acp = value.parse()?,
        "serve.max_restarts" => global.serve.max_restarts = value.parse()?,
        "serve.restart_backoff" => global.serve.restart_backoff = value.parse()?,
        "serve.heartbeat_secs" => global.serve.heartbeat_secs = value.parse()?,
        "validation.type_strictness" => global.validation.type_strictness = value.into(),
        "lint.fix_missing_stubs" => global.lint.fix_missing_stubs = value.parse()?,
        "lint.fix_empty_sections" => global.lint.fix_empty_sections = value.parse()?,
        "logging.log_path" => global.logging.log_path = value.into(),
        "logging.log_rotation" => global.logging.log_rotation = value.into(),
        "logging.log_max_files" => global.logging.log_max_files = value.parse()?,
        "logging.log_format" => global.logging.log_format = value.into(),
        _ => anyhow::bail!("unknown key: {key}"),
    }
    Ok(())
}

/// Set a per-wiki config value. Only keys with per-wiki scope are accepted.
pub fn set_wiki_config_value(wiki_cfg: &mut WikiConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "defaults.search_top_k" => {
            wiki_cfg.defaults.get_or_insert_with(Defaults::default).search_top_k = value.parse()?;
        }
        "defaults.search_excerpt" => {
            wiki_cfg.defaults.get_or_insert_with(Defaults::default).search_excerpt = value.parse()?;
        }
        "defaults.search_sections" => {
            wiki_cfg.defaults.get_or_insert_with(Defaults::default).search_sections = value.parse()?;
        }
        "defaults.page_mode" => {
            wiki_cfg.defaults.get_or_insert_with(Defaults::default).page_mode = value.into();
        }
        "defaults.list_page_size" => {
            wiki_cfg.defaults.get_or_insert_with(Defaults::default).list_page_size = value.parse()?;
        }
        "read.no_frontmatter" => {
            wiki_cfg.read.get_or_insert_with(ReadConfig::default).no_frontmatter = value.parse()?;
        }
        "validation.type_strictness" => {
            wiki_cfg.validation.get_or_insert_with(ValidationConfig::default).type_strictness = value.into();
        }
        "lint.fix_missing_stubs" => {
            wiki_cfg.lint.get_or_insert_with(LintConfig::default).fix_missing_stubs = value.parse()?;
        }
        "lint.fix_empty_sections" => {
            wiki_cfg.lint.get_or_insert_with(LintConfig::default).fix_empty_sections = value.parse()?;
        }
        "global.default_wiki" | "index.auto_rebuild" | "index.auto_recovery"
        | "graph.format" | "graph.depth" | "graph.output"
        | "serve.sse" | "serve.sse_port" | "serve.acp"
        | "serve.max_restarts" | "serve.restart_backoff" | "serve.heartbeat_secs"
        | "logging.log_path" | "logging.log_rotation" | "logging.log_max_files"
        | "logging.log_format" => {
            anyhow::bail!("{key} is a global-only key \u{2014} use --global");
        }
        _ => anyhow::bail!("unknown key: {key}"),
    }
    Ok(())
}
