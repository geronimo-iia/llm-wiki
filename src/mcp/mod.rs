pub mod handlers;
pub mod helpers;
pub mod tools;

use std::future::Future;
use std::sync::{Arc, Mutex};

use rmcp::model::AnnotateAble;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Implementation, ListResourcesResult, ListToolsResult,
    PaginatedRequestParam, RawResource, ReadResourceRequestParam, ReadResourceResult,
    ResourceContents, ResourcesCapability, ServerCapabilities, ServerInfo, ToolsCapability,
};
use rmcp::service::{Peer, RequestContext, RoleServer};
use rmcp::Error as McpError;
use rmcp::ServerHandler;

use crate::engine::{EngineState, WikiEngine};
use crate::markdown;
use crate::slug::{Slug, WikiUri};

// ── McpServer ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct McpServer {
    pub manager: Arc<WikiEngine>,
    pub peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
}

impl McpServer {
    pub fn new(manager: Arc<WikiEngine>) -> Self {
        Self {
            manager,
            peer: Arc::new(Mutex::new(None)),
        }
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
        ServerInfo {
            protocol_version: Default::default(),
            server_info: Implementation {
                name: "llm-wiki".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(true),
                    list_changed: Some(true),
                }),
                ..Default::default()
            },
            instructions: None,
        }
    }

    fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult {
            tools: tools::tool_list(),
            next_cursor: None,
        }))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let args = request.arguments.unwrap_or_default();
        let result = tools::call(self, &request.name, &args);

        // Send resource update notifications for ingested pages
        if !result.notify_uris.is_empty() {
            if let Some(peer) = self.get_peer() {
                let uris = result.notify_uris.clone();
                tokio::spawn(async move {
                    for uri in uris {
                        if let Err(e) = peer
                            .notify_resource_updated(
                                rmcp::model::ResourceUpdatedNotificationParam { uri: uri.clone() },
                            )
                            .await
                        {
                            tracing::warn!(error = %e, uri = %uri, "resource notification failed");
                        }
                    }
                });
            }
        }

        std::future::ready(Ok(CallToolResult {
            content: result.content,
            is_error: Some(result.is_error),
        }))
    }

    fn list_resources(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        let resources = self.list_wiki_resources();
        std::future::ready(Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
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
                        Ok(content) => Ok(ReadResourceResult {
                            contents: vec![ResourceContents::TextResourceContents {
                                uri: uri.to_string(),
                                mime_type: Some("text/markdown".into()),
                                text: content,
                            }],
                        }),
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

    fn get_peer(&self) -> Option<Peer<RoleServer>> {
        self.peer.lock().unwrap().clone()
    }

    fn set_peer(&mut self, peer: Peer<RoleServer>) {
        *self.peer.lock().unwrap() = Some(peer);
    }
}
