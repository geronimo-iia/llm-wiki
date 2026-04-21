pub mod handlers;
pub mod helpers;
pub mod tools;

use std::future::Future;
use std::sync::Arc;

use rmcp::model::AnnotateAble;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Implementation, ListResourcesResult, ListToolsResult,
    PaginatedRequestParams, RawResource, ReadResourceRequestParams, ReadResourceResult,
    ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;

use crate::engine::{EngineState, WikiEngine};
use crate::markdown;
use crate::slug::{Slug, WikiUri};

// ── McpServer ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct McpServer {
    pub manager: Arc<WikiEngine>,
}

impl McpServer {
    pub fn new(manager: Arc<WikiEngine>) -> Self {
        Self { manager }
    }

    pub fn engine(&self) -> std::sync::RwLockReadGuard<'_, EngineState> {
        self.manager.state.read().expect("engine lock poisoned")
    }

    fn list_wiki_resources(&self) -> Vec<rmcp::model::Resource> {
        let engine = match self.manager.state.read() {
            Ok(e) => e,
            Err(_) => return vec![],
        };
        let mut resources = Vec::new();
        for (wiki_name, space) in &engine.spaces {
            let walker = walkdir::WalkDir::new(&space.wiki_root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_file()
                        && e.path().extension().and_then(|x| x.to_str()) == Some("md")
                });
            for entry in walker {
                if let Ok(slug) = Slug::from_path(entry.path(), &space.wiki_root) {
                    let uri = format!("wiki://{wiki_name}/{slug}");
                    resources.push(RawResource::new(uri, slug.title()).no_annotation());
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
            next_cursor: None,
            meta: None,
        }))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let args = request.arguments.unwrap_or_default();
        let result = tools::call(self, &request.name, &args);

        // Send resource update notifications for ingested pages
        if !result.notify_uris.is_empty() {
            let peer = context.peer.clone();
            let uris = result.notify_uris.clone();
            tokio::spawn(async move {
                for uri in uris {
                    if let Err(e) = peer
                        .notify_resource_updated(rmcp::model::ResourceUpdatedNotificationParam {
                            uri: uri.clone(),
                        })
                        .await
                    {
                        tracing::warn!(error = %e, uri = %uri, "resource notification failed");
                    }
                }
            });
        }

        let tool_result = if result.is_error {
            CallToolResult::error(result.content)
        } else {
            CallToolResult::success(result.content)
        };

        std::future::ready(Ok(tool_result))
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        let resources = self.list_wiki_resources();
        std::future::ready(Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        let uri = &request.uri;
        let result = if uri.starts_with("wiki://") {
            let engine = match self.manager.state.read() {
                Ok(e) => e,
                Err(_) => {
                    return std::future::ready(Err(McpError::internal_error(
                        "engine lock poisoned",
                        None,
                    )))
                }
            };
            match WikiUri::resolve(uri, None, &engine.config) {
                Ok((entry, slug)) => {
                    let wiki_root = std::path::PathBuf::from(&entry.path).join("wiki");
                    match markdown::read_page(&slug, &wiki_root, false) {
                        Ok(content) => Ok(ReadResourceResult::new(vec![ResourceContents::text(
                            content,
                            uri.to_string(),
                        )
                        .with_mime_type("text/markdown")])),
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
        std::future::ready(result)
    }
}
