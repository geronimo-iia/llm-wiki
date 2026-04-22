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
    #[serde(default = "default_output_format")]
    pub output_format: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            search_top_k: 10,
            search_excerpt: true,
            search_sections: false,
            page_mode: "flat".into(),
            list_page_size: 20,
            output_format: "text".into(),
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
    #[serde(default = "default_memory_budget_mb")]
    pub memory_budget_mb: u32,
    #[serde(default = "default_tokenizer")]
    pub tokenizer: String,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            auto_rebuild: false,
            auto_recovery: true,
            memory_budget_mb: 50,
            tokenizer: "en_stem".into(),
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
    pub http: bool,
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_http_allowed_hosts")]
    pub http_allowed_hosts: Vec<String>,
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
            http: false,
            http_port: 8080,
            http_allowed_hosts: default_http_allowed_hosts(),
            acp: false,
            max_restarts: 10,
            restart_backoff: 1,
            heartbeat_secs: 60,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    #[serde(default = "default_true")]
    pub auto_commit: bool,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self { auto_commit: true }
    }
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
    pub ingest: IngestConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// A type entry in `[types.<name>]` of `wiki.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeEntry {
    pub schema: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WikiConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub types: std::collections::HashMap<String, TypeEntry>,
    #[serde(default)]
    pub defaults: Option<Defaults>,
    #[serde(default)]
    pub read: Option<ReadConfig>,
    #[serde(default)]
    pub validation: Option<ValidationConfig>,
    #[serde(default)]
    pub ingest: Option<IngestConfig>,
    #[serde(default)]
    pub graph: Option<GraphConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedConfig {
    pub defaults: Defaults,
    pub read: ReadConfig,
    pub index: IndexConfig,
    pub graph: GraphConfig,
    pub serve: ServeConfig,
    pub ingest: IngestConfig,
    pub validation: ValidationConfig,
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
fn default_output_format() -> String {
    "text".into()
}
fn default_memory_budget_mb() -> u32 {
    50
}
fn default_tokenizer() -> String {
    "en_stem".into()
}
fn default_graph_format() -> String {
    "mermaid".into()
}
fn default_graph_depth() -> u32 {
    3
}
fn default_http_port() -> u16 {
    8080
}
fn default_http_allowed_hosts() -> Vec<String> {
    vec!["localhost".into(), "127.0.0.1".into(), "::1".into()]
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
        .join(".llm-wiki")
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
    ResolvedConfig {
        defaults: per_wiki
            .defaults
            .clone()
            .unwrap_or_else(|| global.defaults.clone()),
        read: per_wiki.read.clone().unwrap_or_else(|| global.read.clone()),
        index: global.index.clone(),
        graph: per_wiki
            .graph
            .clone()
            .unwrap_or_else(|| global.graph.clone()),
        serve: global.serve.clone(),
        ingest: per_wiki
            .ingest
            .clone()
            .unwrap_or_else(|| global.ingest.clone()),
        validation: per_wiki
            .validation
            .clone()
            .unwrap_or_else(|| global.validation.clone()),
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

pub fn save_global(config: &GlobalConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn save_wiki(config: &WikiConfig, wiki_root: &Path) -> Result<()> {
    let path = wiki_root.join("wiki.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn set_global_config_value(global: &mut GlobalConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "global.default_wiki" => global.global.default_wiki = value.into(),
        "defaults.search_top_k" => global.defaults.search_top_k = value.parse()?,
        "defaults.search_excerpt" => global.defaults.search_excerpt = value.parse()?,
        "defaults.search_sections" => global.defaults.search_sections = value.parse()?,
        "defaults.page_mode" => global.defaults.page_mode = value.into(),
        "defaults.list_page_size" => global.defaults.list_page_size = value.parse()?,
        "defaults.output_format" => global.defaults.output_format = value.into(),
        "read.no_frontmatter" => global.read.no_frontmatter = value.parse()?,
        "index.auto_rebuild" => global.index.auto_rebuild = value.parse()?,
        "index.auto_recovery" => global.index.auto_recovery = value.parse()?,
        "index.memory_budget_mb" => global.index.memory_budget_mb = value.parse()?,
        "index.tokenizer" => global.index.tokenizer = value.into(),
        "graph.format" => global.graph.format = value.into(),
        "graph.depth" => global.graph.depth = value.parse()?,
        "graph.output" => global.graph.output = value.into(),
        "serve.http" => global.serve.http = value.parse()?,
        "serve.http_port" => global.serve.http_port = value.parse()?,
        "serve.http_allowed_hosts" => {
            global.serve.http_allowed_hosts =
                value.split(',').map(|s| s.trim().to_string()).collect();
        }
        "serve.acp" => global.serve.acp = value.parse()?,
        "serve.max_restarts" => global.serve.max_restarts = value.parse()?,
        "serve.restart_backoff" => global.serve.restart_backoff = value.parse()?,
        "serve.heartbeat_secs" => global.serve.heartbeat_secs = value.parse()?,
        "ingest.auto_commit" => global.ingest.auto_commit = value.parse()?,
        "validation.type_strictness" => global.validation.type_strictness = value.into(),
        "logging.log_path" => global.logging.log_path = value.into(),
        "logging.log_rotation" => global.logging.log_rotation = value.into(),
        "logging.log_max_files" => global.logging.log_max_files = value.parse()?,
        "logging.log_format" => global.logging.log_format = value.into(),
        _ => anyhow::bail!("unknown key: {key}"),
    }
    Ok(())
}

pub fn get_config_value(resolved: &ResolvedConfig, global: &GlobalConfig, key: &str) -> String {
    match key {
        "global.default_wiki" => global.global.default_wiki.clone(),
        "defaults.search_top_k" => resolved.defaults.search_top_k.to_string(),
        "defaults.search_excerpt" => resolved.defaults.search_excerpt.to_string(),
        "defaults.search_sections" => resolved.defaults.search_sections.to_string(),
        "defaults.page_mode" => resolved.defaults.page_mode.clone(),
        "defaults.list_page_size" => resolved.defaults.list_page_size.to_string(),
        "defaults.output_format" => resolved.defaults.output_format.clone(),
        "read.no_frontmatter" => resolved.read.no_frontmatter.to_string(),
        "index.auto_rebuild" => resolved.index.auto_rebuild.to_string(),
        "index.auto_recovery" => global.index.auto_recovery.to_string(),
        "index.memory_budget_mb" => global.index.memory_budget_mb.to_string(),
        "index.tokenizer" => global.index.tokenizer.clone(),
        "graph.format" => resolved.graph.format.clone(),
        "graph.depth" => resolved.graph.depth.to_string(),
        "graph.output" => resolved.graph.output.clone(),
        "serve.http" => resolved.serve.http.to_string(),
        "serve.http_port" => resolved.serve.http_port.to_string(),
        "serve.http_allowed_hosts" => resolved.serve.http_allowed_hosts.join(","),
        "serve.acp" => resolved.serve.acp.to_string(),
        "serve.max_restarts" => global.serve.max_restarts.to_string(),
        "serve.restart_backoff" => global.serve.restart_backoff.to_string(),
        "serve.heartbeat_secs" => global.serve.heartbeat_secs.to_string(),
        "validation.type_strictness" => resolved.validation.type_strictness.clone(),
        "logging.log_path" => global.logging.log_path.clone(),
        "logging.log_rotation" => global.logging.log_rotation.clone(),
        "logging.log_max_files" => global.logging.log_max_files.to_string(),
        "logging.log_format" => global.logging.log_format.clone(),
        "ingest.auto_commit" => resolved.ingest.auto_commit.to_string(),
        _ => format!("unknown key: {key}"),
    }
}

pub fn set_wiki_config_value(wiki_cfg: &mut WikiConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "defaults.search_top_k" => {
            wiki_cfg
                .defaults
                .get_or_insert_with(Defaults::default)
                .search_top_k = value.parse()?;
        }
        "defaults.search_excerpt" => {
            wiki_cfg
                .defaults
                .get_or_insert_with(Defaults::default)
                .search_excerpt = value.parse()?;
        }
        "defaults.search_sections" => {
            wiki_cfg
                .defaults
                .get_or_insert_with(Defaults::default)
                .search_sections = value.parse()?;
        }
        "defaults.page_mode" => {
            wiki_cfg
                .defaults
                .get_or_insert_with(Defaults::default)
                .page_mode = value.into();
        }
        "defaults.list_page_size" => {
            wiki_cfg
                .defaults
                .get_or_insert_with(Defaults::default)
                .list_page_size = value.parse()?;
        }
        "defaults.output_format" => {
            wiki_cfg
                .defaults
                .get_or_insert_with(Defaults::default)
                .output_format = value.into();
        }
        "read.no_frontmatter" => {
            wiki_cfg
                .read
                .get_or_insert_with(ReadConfig::default)
                .no_frontmatter = value.parse()?;
        }
        "validation.type_strictness" => {
            wiki_cfg
                .validation
                .get_or_insert_with(ValidationConfig::default)
                .type_strictness = value.into();
        }
        "ingest.auto_commit" => {
            wiki_cfg
                .ingest
                .get_or_insert_with(IngestConfig::default)
                .auto_commit = value.parse()?;
        }
        "graph.format" => {
            wiki_cfg
                .graph
                .get_or_insert_with(GraphConfig::default)
                .format = value.into();
        }
        "graph.depth" => {
            wiki_cfg
                .graph
                .get_or_insert_with(GraphConfig::default)
                .depth = value.parse()?;
        }
        "graph.output" => {
            wiki_cfg
                .graph
                .get_or_insert_with(GraphConfig::default)
                .output = value.into();
        }
        "global.default_wiki"
        | "index.auto_rebuild"
        | "index.auto_recovery"
        | "index.memory_budget_mb"
        | "index.tokenizer"
        | "serve.http"
        | "serve.http_port"
        | "serve.http_allowed_hosts"
        | "serve.acp"
        | "serve.max_restarts"
        | "serve.restart_backoff"
        | "serve.heartbeat_secs"
        | "logging.log_path"
        | "logging.log_rotation"
        | "logging.log_max_files"
        | "logging.log_format" => {
            anyhow::bail!("{key} is a global-only key \u{2014} use --global");
        }
        _ => anyhow::bail!("unknown key: {key}"),
    }
    Ok(())
}
