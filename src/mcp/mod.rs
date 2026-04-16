pub mod tools;

use std::path::PathBuf;

use rmcp::model::{
    CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult, Implementation,
    ListPromptsResult, ListResourcesResult, ListToolsResult, PaginatedRequestParam, Prompt,
    PromptArgument, PromptMessage, PromptMessageRole, ReadResourceRequestParam, ReadResourceResult,
    ServerCapabilities, ServerInfo,
};
use rmcp::model::{PromptsCapability, ResourceContents, ResourcesCapability, ToolsCapability};
use rmcp::service::{Peer, RequestContext, RoleServer};
use rmcp::Error as McpError;
use rmcp::ServerHandler;

use crate::cli;
use crate::markdown;
use crate::server::WikiServer;

// ── Prompts ───────────────────────────────────────────────────────────────────

fn prompt_list() -> Vec<Prompt> {
    vec![
        Prompt::new(
            "ingest_source",
            Some("Ingest a source document into the wiki"),
            Some(vec![
                PromptArgument {
                    name: "path".into(),
                    description: Some("File or folder path to ingest".into()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "wiki".into(),
                    description: Some("Target wiki name".into()),
                    required: Some(false),
                },
            ]),
        ),
        Prompt::new(
            "research_question",
            Some("Search the wiki and synthesize an answer"),
            Some(vec![PromptArgument {
                name: "question".into(),
                description: Some("Research question to answer".into()),
                required: Some(true),
            }]),
        ),
        Prompt::new(
            "lint_and_fix",
            Some("Run structural lint and fix issues"),
            Some(vec![PromptArgument {
                name: "wiki".into(),
                description: Some("Target wiki name".into()),
                required: Some(false),
            }]),
        ),
    ]
}

fn get_prompt_content(
    name: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> Option<GetPromptResult> {
    let instructions = crate::server::INSTRUCTIONS;
    match name {
        "ingest_source" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("<path>");
            let workflow = cli::extract_workflow(instructions, "ingest").unwrap_or_default();
            Some(GetPromptResult {
                description: Some("Ingest a source document into the wiki".into()),
                messages: vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!("{workflow}\n\nIngest: {path}"),
                )],
            })
        }
        "research_question" => {
            let question = args
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("<question>");
            let workflow = cli::extract_workflow(instructions, "research").unwrap_or_default();
            Some(GetPromptResult {
                description: Some("Search the wiki and synthesize an answer".into()),
                messages: vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!("{workflow}\n\nQuestion: {question}"),
                )],
            })
        }
        "lint_and_fix" => {
            let workflow = cli::extract_workflow(instructions, "lint").unwrap_or_default();
            Some(GetPromptResult {
                description: Some("Run structural lint and fix issues".into()),
                messages: vec![PromptMessage::new_text(PromptMessageRole::User, workflow)],
            })
        }
        _ => None,
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────────

impl ServerHandler for WikiServer {
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
                prompts: Some(PromptsCapability { list_changed: None }),
                ..Default::default()
            },
            instructions: Some(self.instructions.as_ref().clone()),
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

    fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListPromptsResult {
            prompts: prompt_list(),
            next_cursor: None,
        }))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        let args = request.arguments.unwrap_or_default();
        let result = get_prompt_content(&request.name, &args);
        std::future::ready(match result {
            Some(r) => Ok(r),
            None => Err(McpError::invalid_params(
                format!("unknown prompt: {}", request.name),
                None,
            )),
        })
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
        let result = if let Some(stripped) = uri.strip_prefix("wiki://") {
            let global = crate::config::load_global(self.config_path());
            match global {
                Ok(g) => match crate::spaces::resolve_uri(uri, &g) {
                    Ok((entry, slug)) => {
                        let wiki_root = PathBuf::from(&entry.path).join("wiki");
                        match markdown::read_page(&slug, &wiki_root, false) {
                            Ok(content) => Ok(ReadResourceResult {
                                contents: vec![ResourceContents::TextResourceContents {
                                    uri: uri.to_string(),
                                    mime_type: Some("text/markdown".into()),
                                    text: content,
                                }],
                            }),
                            Err(e) => Err(McpError::internal_error(
                                format!("failed to read {stripped}: {e}"),
                                None,
                            )),
                        }
                    }
                    Err(e) => Err(McpError::invalid_params(format!("{e}"), None)),
                },
                Err(e) => Err(McpError::internal_error(format!("{e}"), None)),
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

use std::future::Future;
