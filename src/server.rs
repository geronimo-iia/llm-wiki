//! MCP server — exposes wiki tools, resources, and prompts via `rmcp`.
//!
//! `wiki serve` starts the server in stdio mode (default).
//! SSE transport (`wiki serve --sse`) is stubbed for Phase 6.
//! See `docs/design/design.md` for the full MCP tool and prompt catalogue.

use crate::analysis::PageType;
use crate::context::context as wiki_context_fn;
use crate::ingest;
use crate::integrate;
use crate::lint;
use crate::markdown::resolve_slug;
use crate::registry::WikiRegistry;
use crate::search::{search as wiki_search_fn, search_all};
use rmcp::model::{
    AnnotateAble, CallToolResult, Content, GetPromptRequestParam, GetPromptResult,
    ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParam,
    Prompt, PromptMessage, PromptMessageRole, RawResource, RawResourceTemplate,
    ReadResourceRequestParam, ReadResourceResult, ResourceContents, ResourceTemplate,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{tool, Error as McpError, ServerHandler};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ── Serialisable return types ─────────────────────────────────────────────────

/// Summary of a single wiki page, returned by `wiki_list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    pub slug: String,
    pub title: String,
    pub page_type: String,
}

/// Serialisable lint report summary for MCP callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintSummary {
    pub orphan_count: usize,
    pub orphans: Vec<String>,
    pub missing_stub_count: usize,
    pub missing_stubs: Vec<String>,
    pub active_contradiction_count: usize,
    pub active_contradictions: Vec<String>,
}

// ── WikiServer ────────────────────────────────────────────────────────────────

/// MCP server that wraps the wiki engine.
///
/// Exposes `wiki_ingest`, `wiki_context`, `wiki_search`, `wiki_lint`, and
/// `wiki_list` as MCP tools, plus named workflow prompts and resource reading.
///
/// Phase 6: optionally holds a `WikiRegistry` for multi-wiki operations.
/// All existing single-wiki behaviour is preserved when no registry is set.
#[derive(Clone)]
pub struct WikiServer {
    /// Default path to the wiki root directory (single-wiki mode / fallback).
    pub wiki_root: PathBuf,
    /// Optional multi-wiki registry (Phase 6).
    pub registry: Option<Arc<WikiRegistry>>,
    /// Peer handle for sending resource-updated notifications.
    peer: Arc<Mutex<Option<rmcp::service::Peer<RoleServer>>>>,
}

impl WikiServer {
    /// Create a new `WikiServer` rooted at `wiki_root`.
    ///
    /// This constructor keeps the single-wiki behaviour intact — all existing
    /// tests in `tests/mcp.rs` use it and must continue to pass.
    pub fn new(wiki_root: PathBuf) -> Self {
        Self {
            wiki_root,
            registry: None,
            peer: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a `WikiServer` with a multi-wiki registry.
    ///
    /// `wiki_root` is the fallback root when the registry cannot resolve a name.
    pub fn new_with_registry(wiki_root: PathBuf, registry: Arc<WikiRegistry>) -> Self {
        Self {
            wiki_root,
            registry: Some(registry),
            peer: Arc::new(Mutex::new(None)),
        }
    }

    /// Resolve the root directory for a given optional wiki name.
    ///
    /// Resolution order:
    /// 1. If `wiki` is `Some(name)` and a registry is configured, look up by name.
    /// 2. If `wiki` is `None` and a registry is configured, return the default wiki.
    /// 3. Fall back to `self.wiki_root` in all other cases (single-wiki mode,
    ///    or when the registry lookup fails).
    fn resolve_root(&self, wiki: Option<&str>) -> PathBuf {
        if let Some(reg) = &self.registry {
            if let Ok(config) = reg.resolve(wiki) {
                return config.root.clone();
            }
        }
        self.wiki_root.clone()
    }

    /// Ingest into a specific wiki (identified by optional name).
    ///
    /// This is the multi-wiki variant of [`do_ingest`]; the single-wiki version
    /// is preserved for backward compatibility with all existing tests.
    pub fn do_ingest_with_wiki(
        &self,
        analysis_value: serde_json::Value,
        wiki: Option<&str>,
    ) -> Result<String, String> {
        let root = self.resolve_root(wiki);
        // Reuse do_ingest_root to avoid duplicating logic.
        Self::do_ingest_at(analysis_value, &root)
    }

    /// Core ingest logic, parameterised over the target root path.
    fn do_ingest_at(analysis_value: serde_json::Value, root: &Path) -> Result<String, String> {
        let json = serde_json::to_string(&analysis_value)
            .map_err(|e| format!("failed to serialise analysis: {e}"))?;

        let analysis = ingest::parse_analysis(&json).map_err(|e| e.to_string())?;

        crate::git::init_if_needed(root).map_err(|e| e.to_string())?;

        let report = integrate::integrate(analysis, root).map_err(|e| e.to_string())?;

        let commit_msg = format!("ingest: {} — +{} pages", report.title, report.total_pages());
        crate::git::commit(root, &commit_msg).map_err(|e| e.to_string())?;

        Ok(format!(
            "Ingested: {}\n  created:        {}\n  updated:        {}\n  appended:       {}\n  contradictions: {}",
            report.title,
            report.pages_created,
            report.pages_updated,
            report.pages_appended,
            report.contradictions_written,
        ))
    }

    // ── Public helpers (called by tool methods and by tests directly) ─────────

    /// Ingest an `analysis.json` value into the wiki.
    ///
    /// Uses `self.wiki_root` — single-wiki mode.  This signature is preserved
    /// so that all existing tests in `tests/mcp.rs` continue to compile and pass.
    pub fn do_ingest(&self, analysis_value: serde_json::Value) -> Result<String, String> {
        Self::do_ingest_at(analysis_value, &self.wiki_root)
    }

    /// Return context Markdown for a question (top-k pages).
    pub fn do_context(&self, question: &str, top_k: usize) -> String {
        wiki_context_fn(question, &self.wiki_root, top_k).unwrap_or_default()
    }

    /// List all wiki pages, optionally filtered by type string.
    pub fn do_list_pages(&self, page_type_filter: Option<&str>) -> Vec<PageSummary> {
        Self::list_pages_in_root(&self.wiki_root, page_type_filter)
    }

    /// List pages in an arbitrary wiki root (used for multi-wiki resource enumeration).
    fn list_pages_in_root(wiki_root: &Path, page_type_filter: Option<&str>) -> Vec<PageSummary> {
        let filter: Option<PageType> = page_type_filter.and_then(|s| match s {
            "concept" => Some(PageType::Concept),
            "source" | "source-summary" => Some(PageType::SourceSummary),
            "contradiction" => Some(PageType::Contradiction),
            "query" | "query-result" => Some(PageType::QueryResult),
            _ => None,
        });

        let mut summaries = Vec::new();

        for entry in walkdir::WalkDir::new(wiki_root).follow_links(false) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension() != Some(OsStr::new("md")) {
                continue;
            }
            let rel = match path.strip_prefix(wiki_root) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let rel_str = rel.to_string_lossy();
            if rel_str.starts_with(".wiki") || rel_str.starts_with("raw") {
                continue;
            }
            if rel == std::path::Path::new("LINT.md") {
                continue;
            }
            // Skip non-index.md files inside bundle folders.
            if let Some(filename) = path.file_name() {
                if filename != OsStr::new("index.md") {
                    if let Some(parent) = path.parent() {
                        if parent.join("index.md").exists() {
                            continue;
                        }
                    }
                }
            }

            let slug = crate::markdown::slug_for(path, wiki_root);
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let yaml = match Self::yaml_block(&content) {
                Some(y) => y,
                None => continue,
            };

            let page_type = Self::page_type_from_yaml(&yaml);

            if let Some(ref ft) = filter {
                if page_type.as_ref() != Some(ft) {
                    continue;
                }
            }

            let title = Self::title_from_yaml(&yaml);
            let type_str = match page_type {
                Some(PageType::Concept) => "concept",
                Some(PageType::SourceSummary) => "source-summary",
                Some(PageType::QueryResult) => "query-result",
                Some(PageType::Contradiction) => "contradiction",
                None => "unknown",
            };
            summaries.push(PageSummary {
                slug,
                title,
                page_type: type_str.to_string(),
            });
        }

        summaries.sort_by(|a, b| a.slug.cmp(&b.slug));
        summaries
    }

    /// Read a wiki page or bundle asset by resource URI.
    ///
    /// Supports:
    /// - `wiki://{wiki}/{slug}` — page (flat or bundle, resolved via `resolve_slug`)
    /// - `wiki://{wiki}/{slug}/{filename}` — bundle asset co-located with a page
    pub fn do_read_resource(&self, uri: &str) -> Result<String, McpError> {
        let rest = uri
            .strip_prefix("wiki://")
            .ok_or_else(|| McpError::resource_not_found("invalid URI scheme", None))?;

        let slash = rest
            .find('/')
            .ok_or_else(|| McpError::resource_not_found("URI missing path", None))?;
        let wiki_name = &rest[..slash];
        let slug = &rest[slash + 1..];

        if slug.is_empty() {
            return Err(McpError::resource_not_found("empty slug", None));
        }

        // Resolve the root.
        let root = if let Some(reg) = &self.registry {
            match reg.resolve(Some(wiki_name)) {
                Ok(config) => config.root.clone(),
                Err(_) => {
                    return Err(McpError::resource_not_found(
                        format!("wiki '{}' not found", wiki_name),
                        None,
                    ))
                }
            }
        } else {
            self.wiki_root.clone()
        };

        // Try bundle asset: if slug has the form `{page_slug}/{filename}` where
        // filename has no further slashes and is not a .md file, treat as asset.
        let parts: Vec<&str> = slug.splitn(3, '/').collect();
        if parts.len() >= 2 {
            let last = *parts.last().unwrap();
            // If the last component looks like a non-md file (has an extension)
            // and the path without the last component resolves as a bundle, read the asset.
            if last.contains('.') && !last.ends_with(".md") {
                let page_slug = &slug[..slug.rfind('/').unwrap()];
                let asset_path = root.join(page_slug).join(last);
                if asset_path.exists() {
                    return std::fs::read_to_string(&asset_path).map_err(|_| {
                        McpError::resource_not_found(
                            format!("asset not found: {slug}"),
                            None,
                        )
                    });
                }
            }
        }

        // Validate type prefix for page resources.
        let valid = ["concepts/", "sources/", "contradictions/", "queries/", "assets/"];
        if !valid.iter().any(|p| slug.starts_with(p)) {
            return Err(McpError::resource_not_found("unknown resource type", None));
        }

        // Resolve flat or bundle page.
        let path = match resolve_slug(&root, slug) {
            Some(p) => p,
            None => {
                return Err(McpError::resource_not_found(
                    format!("page not found: {slug}"),
                    None,
                ))
            }
        };
        std::fs::read_to_string(&path)
            .map_err(|_| McpError::resource_not_found(format!("page not found: {slug}"), None))
    }

    // ── Private YAML helpers ──────────────────────────────────────────────────

    fn yaml_block(content: &str) -> Option<String> {
        let after_open = content.strip_prefix("---\n")?;
        let end = after_open.find("\n---\n")?;
        Some(after_open[..end].to_string())
    }

    fn page_type_from_yaml(yaml: &str) -> Option<PageType> {
        let val: serde_yaml::Value = serde_yaml::from_str(yaml).ok()?;
        match val.get("type")?.as_str()? {
            "concept" => Some(PageType::Concept),
            "source-summary" => Some(PageType::SourceSummary),
            "query-result" => Some(PageType::QueryResult),
            "contradiction" => Some(PageType::Contradiction),
            _ => None,
        }
    }

    fn title_from_yaml(yaml: &str) -> String {
        serde_yaml::from_str::<serde_yaml::Value>(yaml)
            .ok()
            .and_then(|v| v.get("title")?.as_str().map(str::to_string))
            .unwrap_or_default()
    }
}

// ── MCP Tools ─────────────────────────────────────────────────────────────────

#[tool(tool_box)]
impl WikiServer {
    /// Integrate an Analysis JSON document into the wiki.
    #[tool(description = "Ingest an analysis.json document into the wiki. Pass the full JSON object (not a file path). The optional `wiki` param targets a named wiki from the registry. Returns a summary of changes written.")]
    fn wiki_ingest(
        &self,
        #[tool(param)] analysis: serde_json::Value,
        #[tool(param)] wiki: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        match self.do_ingest_with_wiki(analysis, wiki.as_deref()) {
            Ok(summary) => Ok(CallToolResult::success(vec![Content::text(summary)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    /// Return the top-K relevant wiki pages as Markdown for an external LLM.
    #[tool(description = "Return the top-K relevant wiki pages as Markdown context for a question. Contradiction pages are included automatically.")]
    fn wiki_context(
        &self,
        #[tool(param)] question: String,
        #[tool(param)] wiki: Option<String>,
        #[tool(param)] top_k: Option<u32>,
    ) -> Result<CallToolResult, McpError> {
        let k = top_k.unwrap_or(5) as usize;
        let root = self.resolve_root(wiki.as_deref());
        let md = wiki_context_fn(&question, &root, k).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(md)]))
    }

    /// Full-text BM25 search across wiki pages.
    #[tool(description = "Full-text search across wiki pages. Set `all_wikis: true` to search all registered wikis. Returns a JSON array ordered by relevance.")]
    fn wiki_search(
        &self,
        #[tool(param)] query: String,
        #[tool(param)] wiki: Option<String>,
        #[tool(param)] all_wikis: Option<bool>,
    ) -> Result<CallToolResult, McpError> {
        if all_wikis.unwrap_or(false) {
            if let Some(reg) = &self.registry {
                let results = search_all(reg, &query, 20).unwrap_or_default();

                #[derive(Serialize)]
                struct RowAll<'a> {
                    wiki_name: &'a str,
                    slug: &'a str,
                    title: &'a str,
                    snippet: &'a str,
                    score: f32,
                    page_type: &'a str,
                }
                let rows: Vec<RowAll<'_>> = results
                    .iter()
                    .map(|r| RowAll {
                        wiki_name: &r.wiki_name,
                        slug: &r.slug,
                        title: &r.title,
                        snippet: &r.snippet,
                        score: r.score,
                        page_type: &r.page_type,
                    })
                    .collect();
                let json = serde_json::to_string_pretty(&rows).unwrap_or_default();
                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }
        }

        let root = self.resolve_root(wiki.as_deref());
        let results = wiki_search_fn(&query, &root, false).unwrap_or_default();

        #[derive(Serialize)]
        struct Row<'a> {
            slug: &'a str,
            title: &'a str,
            snippet: &'a str,
            score: f32,
            page_type: &'a str,
        }
        let rows: Vec<Row<'_>> = results
            .iter()
            .map(|r| Row {
                slug: &r.slug,
                title: &r.title,
                snippet: &r.snippet,
                score: r.score,
                page_type: &r.page_type,
            })
            .collect();
        let json = serde_json::to_string_pretty(&rows).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Structural lint pass: orphan pages, missing stubs, active contradictions.
    #[tool(description = "Run a structural lint pass: orphans, missing stubs, active contradictions. Writes LINT.md and commits it. Returns a JSON summary.")]
    fn wiki_lint(
        &self,
        #[tool(param)] wiki: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.resolve_root(wiki.as_deref());
        match lint::lint(&root) {
            Ok(report) => {
                let summary = LintSummary {
                    orphan_count: report.orphan_pages.len(),
                    orphans: report.orphan_pages.clone(),
                    missing_stub_count: report.missing_stubs.len(),
                    missing_stubs: report.missing_stubs.clone(),
                    active_contradiction_count: report.active_contradictions.len(),
                    active_contradictions: report
                        .active_contradictions
                        .iter()
                        .map(|c| c.slug.clone())
                        .collect(),
                };
                let json = serde_json::to_string_pretty(&summary).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    /// List wiki pages, optionally filtered by type.
    #[tool(description = "List wiki pages, optionally filtered by type (concept|source|contradiction|query). Returns a JSON array of {slug, title, page_type} objects.")]
    fn wiki_list(
        &self,
        #[tool(param)] wiki: Option<String>,
        #[tool(param)] page_type: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.resolve_root(wiki.as_deref());
        let summaries = Self::list_pages_in_root(&root, page_type.as_deref());
        let json = serde_json::to_string_pretty(&summaries).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// ── MCP ServerHandler (resources + prompts; tools wired by #[tool(tool_box)]) ─

#[tool(tool_box)]
impl ServerHandler for WikiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(include_str!("instructions.md").into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            ..Default::default()
        }
    }

    fn get_peer(&self) -> Option<rmcp::service::Peer<RoleServer>> {
        self.peer.lock().ok()?.clone()
    }

    fn set_peer(&mut self, peer: rmcp::service::Peer<RoleServer>) {
        if let Ok(mut guard) = self.peer.lock() {
            *guard = Some(peer);
        }
    }

    // ── Resources ─────────────────────────────────────────────────────────────

    async fn list_resource_templates(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        // Multi-wiki: one template per registered wiki.
        if let Some(reg) = &self.registry {
            let templates: Vec<_> = reg
                .entries()
                .iter()
                .map(|e| {
                    RawResourceTemplate {
                        uri_template: format!("wiki://{}/{{type}}/{{slug}}", e.name),
                        name: format!("{} wiki page", e.name),
                        description: Some(format!("A page in the '{}' wiki", e.name)),
                        mime_type: Some("text/markdown".into()),
                    }
                    .no_annotation()
                })
                .collect();
            return Ok(ListResourceTemplatesResult {
                resource_templates: templates,
                next_cursor: None,
            });
        }

        // Single-wiki fallback (Phase 4 behaviour).
        let template = RawResourceTemplate {
            uri_template: "wiki://default/{type}/{slug}".into(),
            name: "wiki page".into(),
            description: Some("A wiki page at the given type/slug path".into()),
            mime_type: Some("text/markdown".into()),
        };
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![template.no_annotation()],
            next_cursor: None,
        })
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        // Multi-wiki: enumerate pages from all registered wikis.
        if let Some(reg) = &self.registry {
            let mut resources = Vec::new();
            for entry in reg.entries() {
                let summaries = Self::list_pages_in_root(&entry.path, None);
                for s in summaries {
                    resources.push(
                        RawResource::new(
                            format!("wiki://{}/{}", entry.name, s.slug),
                            s.title,
                        )
                        .no_annotation(),
                    );
                }
            }
            return Ok(ListResourcesResult {
                resources,
                next_cursor: None,
            });
        }

        // Single-wiki fallback (Phase 4 behaviour).
        let summaries = self.do_list_pages(None);
        let resources: Vec<_> = summaries
            .into_iter()
            .map(|s| {
                RawResource::new(format!("wiki://default/{}", s.slug), s.title).no_annotation()
            })
            .collect();
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri.as_str();
        let content = self.do_read_resource(uri)?;
        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, uri)],
        })
    }

    // ── Prompts ───────────────────────────────────────────────────────────────

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: vec![
                Prompt::new(
                    "ingest_source",
                    Some("Step-by-step workflow to ingest a new source into the wiki"),
                    None,
                ),
                Prompt::new(
                    "research_question",
                    Some("Retrieve wiki context and synthesise an answer to a question"),
                    None,
                ),
                Prompt::new("lint_and_enrich", Some("Run lint and address findings"), None),
                Prompt::new(
                    "analyse_contradiction",
                    Some("Deep-dive into a specific contradiction page"),
                    None,
                ),
            ],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let args = request.arguments.unwrap_or_default();
        let get_str = |key: &str| -> String {
            args.get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        match request.name.as_str() {
            "ingest_source" => {
                let source = get_str("source");
                let msg = format!(
                    "Ingest the following source into the wiki.\n\n\
                     Source: {source}\n\n\
                     Steps:\n\
                     1. Read and analyse the source document.\n\
                     2. Call `wiki_context` with key concepts from the source to check for \
                        existing pages.\n\
                     3. Produce an `analysis.json` object matching the wiki schema (see \
                        server instructions).\n\
                     4. Call `wiki_ingest(analysis: <your json>)` to write the pages.\n\
                     5. Review the ingest summary. If contradictions were found, inspect \
                        them with `wiki_context`."
                );
                Ok(GetPromptResult {
                    description: Some("Ingest a new source into the wiki".into()),
                    messages: vec![PromptMessage::new_text(PromptMessageRole::User, msg)],
                })
            }

            "research_question" => {
                let question = get_str("question");
                let save = args.get("save").and_then(|v| v.as_bool()).unwrap_or(false);
                let save_hint = if save {
                    " Include a `query-result` page in your analysis.json and call \
                     `wiki_ingest` to save the answer."
                } else {
                    ""
                };
                let msg = format!(
                    "Answer the following question using the wiki.\n\n\
                     Question: {question}\n\n\
                     Steps:\n\
                     1. Call `wiki_context(question: \"{question}\")` to retrieve relevant \
                        pages.\n\
                     2. Synthesise an answer from the returned Markdown. Cite page slugs.\n\
                     3. Note any contradiction pages — they capture knowledge \
                        structure.{save_hint}"
                );
                Ok(GetPromptResult {
                    description: Some("Retrieve wiki context and answer a question".into()),
                    messages: vec![PromptMessage::new_text(PromptMessageRole::User, msg)],
                })
            }

            "lint_and_enrich" => {
                let msg = "Run a lint pass on the wiki and address findings.\n\n\
                     Steps:\n\
                     1. Call `wiki_lint()` to audit the wiki.\n\
                     2. For each missing stub, identify a source covering that concept and \
                        ingest it.\n\
                     3. For each orphan page, add cross-references or merge it.\n\
                     4. For each active contradiction, call `wiki_context` with the \
                        contradiction title and evaluate whether a resolution is possible.\n\
                     5. Re-run `wiki_lint()` to confirm findings are resolved."
                    .to_string();
                Ok(GetPromptResult {
                    description: Some("Run lint and address findings".into()),
                    messages: vec![PromptMessage::new_text(PromptMessageRole::User, msg)],
                })
            }

            "analyse_contradiction" => {
                let slug = get_str("slug");
                let msg = format!(
                    "Deep-dive into the contradiction at `{slug}`.\n\n\
                     Steps:\n\
                     1. Call `wiki_context(question: \"{slug}\")` to retrieve the \
                        contradiction page and related concepts.\n\
                     2. Read the `dimension` and `epistemic_value` fields carefully.\n\
                     3. Identify what each source claims and why they disagree.\n\
                     4. If a resolution is possible, produce an `analysis.json` with \
                        `action: update` on `{slug}`, set `status: resolved`, and add a \
                        `resolution` field.\n\
                     5. Call `wiki_ingest` to write the resolution."
                );
                Ok(GetPromptResult {
                    description: Some("Analyse a contradiction page".into()),
                    messages: vec![PromptMessage::new_text(PromptMessageRole::User, msg)],
                })
            }

            name => Err(McpError::invalid_params(
                format!("unknown prompt: {name}"),
                None,
            )),
        }
    }
}
