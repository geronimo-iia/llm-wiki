#![allow(unreachable_pub)]
/// MCP tool handler functions.
pub mod handlers;
/// MCP helper utilities — argument extraction and tool result types.
pub mod helpers;
/// MCP tool definitions and dispatch table.
pub mod tools;

use std::future::Future;
use std::sync::Arc;

use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse,
    ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};

use crate::engine::{EngineState, WikiEngine};
use crate::markdown;
use crate::slug::{Slug, WikiUri};

// ── McpServer ─────────────────────────────────────────────────────────────────

/// MCP server — dispatches MCP tool calls and resource reads to the wiki engine.
#[derive(Clone)]
pub struct McpServer {
    /// Shared wiki engine handle.
    pub manager: Arc<WikiEngine>,
}

impl McpServer {
    /// Create a new `McpServer` wrapping `manager`.
    pub fn new(manager: Arc<WikiEngine>) -> Self {
        Self { manager }
    }

    /// Acquire a read guard on the engine state.
    pub fn engine(&self) -> Result<std::sync::RwLockReadGuard<'_, EngineState>, String> {
        self.manager
            .state
            .read()
            .map_err(|_| "engine lock poisoned".to_string())
    }

    fn list_wiki_resources(&self) -> Vec<rmcp::model::Resource> {
        let roots: Vec<(String, std::path::PathBuf)> = match self.manager.state.read() {
            Ok(engine) => engine
                .spaces
                .iter()
                .map(|(name, space)| (name.clone(), space.wiki_root.clone()))
                .collect(),
            Err(_) => return vec![],
        };
        let mut resources = Vec::new();
        for (wiki_name, wiki_root) in &roots {
            let walker = walkdir::WalkDir::new(wiki_root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_file()
                        && e.path().extension().and_then(|x| x.to_str()) == Some("md")
                });
            for entry in walker {
                if let Ok(slug) = Slug::from_path(entry.path(), wiki_root) {
                    let uri = format!("wiki://{wiki_name}/{slug}");
                    resources.push(Resource::new(uri, slug.title()));
                }
            }
        }
        resources
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────────

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_list_changed()
                .build(),
        )
        .with_server_info(Implementation::new("llm-wiki", env!("CARGO_PKG_VERSION")))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult {
            tools: tools::tool_list(),
            ..Default::default()
        }))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let args = request.arguments.unwrap_or_default();
        let result = tokio::task::block_in_place(|| tools::call(self, &request.name, &args));

        // Send resource update notifications for ingested pages
        if !result.notify_uris.is_empty() {
            let peer = context.peer.clone();
            let uris = result.notify_uris.clone();
            let tool_name = request.name.clone();
            tokio::spawn(async move {
                for uri in uris {
                    if let Err(e) = peer
                        .notify_resource_updated(
                            rmcp::model::ResourceUpdatedNotificationParam::new(uri.clone()),
                        )
                        .await
                    {
                        tracing::warn!(tool = %tool_name, uri = %uri, error = %e, "resource notification failed");
                    }
                }
            });
        }

        // Send resource list changed notification for space operations
        if result.notify_resources_changed {
            let peer = context.peer.clone();
            let tool_name = request.name.clone();
            tokio::spawn(async move {
                if let Err(e) = peer.notify_resource_list_changed().await {
                    tracing::warn!(tool = %tool_name, error = %e, "resource list changed notification failed");
                }
            });
        }

        let tool_result = if result.is_error {
            CallToolResult::error(result.content)
        } else {
            CallToolResult::success(result.content)
        };

        Ok(tool_result.into())
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        let resources = self.list_wiki_resources();
        std::future::ready(Ok(ListResourcesResult {
            resources,
            ..Default::default()
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResponse, McpError>> + Send + '_ {
        let uri = &request.uri;
        let result = if uri.starts_with("wiki://") {
            let engine = match self.manager.state.read() {
                Ok(e) => e,
                Err(_) => {
                    return std::future::ready(Err(McpError::internal_error(
                        "engine lock poisoned",
                        None,
                    )));
                }
            };
            match WikiUri::resolve(uri, None, &engine.config) {
                Ok((entry, slug)) => {
                    let wiki_root = engine
                        .space(&entry.name)
                        .map(|s| s.wiki_root.clone())
                        .unwrap_or_else(|_| entry.path.clone().join("wiki"));
                    match markdown::read_page(&slug, &wiki_root, false) {
                        Ok(content) => Ok(ReadResourceResult::new(vec![
                            ResourceContents::text(content, uri.to_string())
                                .with_mime_type("text/markdown"),
                        ])),
                        Err(e) => Err(McpError::internal_error(
                            format!("failed to read: {e}"),
                            None,
                        )),
                    }
                }
                Err(e) => Err(McpError::invalid_params(format!("{e}"), None)),
            }
        } else {
            Err(McpError::invalid_params(
                format!("unsupported URI scheme: {uri}"),
                None,
            ))
        };
        std::future::ready(result.map(Into::into))
    }
}
