use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{self as acp, Client as _};
use anyhow::Result;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::engine::WikiEngine;
use crate::ops;

// ── Session ───────────────────────────────────────────────────────────────────

pub struct AcpSession {
    pub id: String,
    pub label: Option<String>,
    pub wiki: Option<String>,
    pub created_at: u64,
    pub active_run: Option<String>,
}

// ── WikiAgent ─────────────────────────────────────────────────────────────────

pub struct WikiAgent {
    pub manager: Arc<WikiEngine>,
    sessions: Mutex<HashMap<String, AcpSession>>,
    update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
}

impl WikiAgent {
    pub fn new(
        manager: Arc<WikiEngine>,
        update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        Self {
            manager,
            sessions: Mutex::new(HashMap::new()),
            update_tx,
        }
    }

    pub fn resolve_wiki_name(&self, session_wiki: Option<&str>) -> String {
        let engine = self.manager.state.read().expect("engine lock poisoned");
        engine.resolve_wiki_name(session_wiki).to_string()
    }

    fn session_cwd(&self) -> PathBuf {
        let engine = self.manager.state.read().expect("engine lock poisoned");
        let name = engine.default_wiki_name();
        engine
            .space(name)
            .map(|s| s.repo_root.clone())
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    // ── Dispatch ──────────────────────────────────────────────────────────

    /// Parse `llm-wiki:<workflow> <text>` or fall back to keyword matching.
    pub fn dispatch_workflow(prompt: &str) -> (&str, &str) {
        if let Some(rest) = prompt.strip_prefix("llm-wiki:") {
            let rest = rest.trim_start();
            if let Some(pos) = rest.find(char::is_whitespace) {
                let workflow = &rest[..pos];
                let text = rest[pos..].trim_start();
                return (workflow, text);
            }
            return (rest, "");
        }
        // Keyword fallback — default to research
        ("research", prompt)
    }

    fn extract_prompt_text(req: &acp::PromptRequest) -> String {
        req.prompt
            .iter()
            .filter_map(|block| match block {
                acp::ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    // ── Streaming helpers ─────────────────────────────────────────────────

    pub async fn send_message(
        &self,
        session_id: &acp::SessionId,
        text: &str,
    ) -> std::result::Result<(), acp::Error> {
        let notif = acp::SessionNotification::new(
            session_id.clone(),
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new(text),
            ))),
        );
        self.send_notification(notif).await
    }

    pub async fn send_tool_call(
        &self,
        session_id: &acp::SessionId,
        id: &str,
        title: &str,
        kind: acp::ToolKind,
    ) -> std::result::Result<(), acp::Error> {
        let notif = acp::SessionNotification::new(
            session_id.clone(),
            acp::SessionUpdate::ToolCall(
                acp::ToolCall::new(acp::ToolCallId::new(id), title)
                    .kind(kind)
                    .status(acp::ToolCallStatus::InProgress),
            ),
        );
        self.send_notification(notif).await
    }

    pub async fn send_tool_result(
        &self,
        session_id: &acp::SessionId,
        id: &str,
        status: acp::ToolCallStatus,
        content: &str,
    ) -> std::result::Result<(), acp::Error> {
        let notif = acp::SessionNotification::new(
            session_id.clone(),
            acp::SessionUpdate::ToolCallUpdate(acp::ToolCallUpdate::new(
                acp::ToolCallId::new(id),
                acp::ToolCallUpdateFields::new()
                    .status(status)
                    .content(vec![content.into()]),
            )),
        );
        self.send_notification(notif).await
    }

    pub fn make_tool_id(workflow: &str, step: &str) -> String {
        format!(
            "{workflow}-{step}-{}",
            chrono::Utc::now().timestamp_millis()
        )
    }

    async fn send_notification(
        &self,
        notif: acp::SessionNotification,
    ) -> std::result::Result<(), acp::Error> {
        let (tx, rx) = oneshot::channel();
        self.update_tx
            .send((notif, tx))
            .map_err(|_| acp::Error::internal_error())?;
        rx.await.map_err(|_| acp::Error::internal_error())
    }

    fn clear_active_run(&self, session_id: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(sess) = sessions.get_mut(session_id) {
                sess.active_run = None;
            }
        }
    }

    // ── Reusable workflow steps ─────────────────────────────────────────

    async fn step_search(
        &self,
        session_id: &acp::SessionId,
        workflow: &str,
        query: &str,
        wiki_name: &str,
        top_k: usize,
    ) -> std::result::Result<Vec<crate::search::PageRef>, acp::Error> {
        let tool_id = Self::make_tool_id(workflow, "search");
        self.send_tool_call(
            session_id,
            &tool_id,
            &format!("wiki_search: {query}"),
            acp::ToolKind::Search,
        )
        .await?;

        let results = {
            let engine = self
                .manager
                .state
                .read()
                .map_err(|_| acp::Error::internal_error())?;
            ops::search(
                &engine,
                wiki_name,
                &ops::SearchParams {
                    query,
                    type_filter: None,
                    no_excerpt: false,
                    top_k: Some(top_k),
                    include_sections: false,
                    cross_wiki: false,
                },
            )
        };

        match results {
            Ok(results) => {
                self.send_tool_result(
                    session_id,
                    &tool_id,
                    acp::ToolCallStatus::Completed,
                    &format!("{} results", results.len()),
                )
                .await?;
                Ok(results)
            }
            Err(e) => {
                self.send_tool_result(
                    session_id,
                    &tool_id,
                    acp::ToolCallStatus::Failed,
                    &format!("{e}"),
                )
                .await?;
                Ok(Vec::new())
            }
        }
    }

    async fn step_read(
        &self,
        session_id: &acp::SessionId,
        workflow: &str,
        slug: &str,
        wiki_name: &str,
    ) -> std::result::Result<(), acp::Error> {
        let tool_id = Self::make_tool_id(workflow, "read");
        self.send_tool_call(
            session_id,
            &tool_id,
            &format!("wiki_content_read: {slug}"),
            acp::ToolKind::Read,
        )
        .await?;

        let result = {
            let engine = self
                .manager
                .state
                .read()
                .map_err(|_| acp::Error::internal_error())?;
            ops::content_read(&engine, slug, Some(wiki_name), false, false)
        };

        match result {
            Ok(_) => {
                self.send_tool_result(
                    session_id,
                    &tool_id,
                    acp::ToolCallStatus::Completed,
                    "",
                )
                .await?;
            }
            Err(e) => {
                self.send_tool_result(
                    session_id,
                    &tool_id,
                    acp::ToolCallStatus::Failed,
                    &format!("{e}"),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn step_report_results(
        &self,
        session_id: &acp::SessionId,
        results: &[crate::search::PageRef],
        wiki_name: &str,
    ) -> std::result::Result<(), acp::Error> {
        if results.is_empty() {
            return Ok(());
        }
        let hits: Vec<String> = results
            .iter()
            .take(5)
            .map(|r| format!("- {} (score: {:.2})", r.uri, r.score))
            .collect();
        self.send_message(
            session_id,
            &format!(
                "Based on {} pages in \"{wiki_name}\":\n{}",
                results.len(),
                hits.join("\n")
            ),
        )
        .await
    }

    // ── Workflows ─────────────────────────────────────────────────────────

    async fn run_research(
        &self,
        session_id: &acp::SessionId,
        query: &str,
        wiki_name: &str,
    ) -> std::result::Result<acp::PromptResponse, acp::Error> {
        self.send_message(session_id, &format!("Searching for: {query}..."))
            .await?;

        let results = self.step_search(session_id, "research", query, wiki_name, 5).await?;

        if results.is_empty() {
            self.send_message(
                session_id,
                &format!("No results found for \"{query}\" in wiki \"{wiki_name}\"."),
            )
            .await?;
        } else {
            self.step_read(session_id, "research", &results[0].slug, wiki_name).await?;
            self.step_report_results(session_id, &results, wiki_name).await?;
        }

        self.clear_active_run(&session_id.to_string());
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }
}

// ── Agent trait impl ──────────────────────────────────────────────────────────

#[async_trait::async_trait(?Send)]
impl acp::Agent for WikiAgent {
    async fn initialize(
        &self,
        _args: acp::InitializeRequest,
    ) -> std::result::Result<acp::InitializeResponse, acp::Error> {
        Ok(acp::InitializeResponse::new(acp::ProtocolVersion::LATEST)
            .agent_capabilities(
                acp::AgentCapabilities::new()
                    .load_session(true)
                    .prompt_capabilities(acp::PromptCapabilities::new())
                    .session_capabilities(
                        acp::SessionCapabilities::new().list(acp::SessionListCapabilities::new()),
                    ),
            )
            .agent_info(acp::Implementation::new(
                "llm-wiki",
                env!("CARGO_PKG_VERSION"),
            )))
    }

    async fn authenticate(
        &self,
        _args: acp::AuthenticateRequest,
    ) -> std::result::Result<acp::AuthenticateResponse, acp::Error> {
        Ok(acp::AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        req: acp::NewSessionRequest,
    ) -> std::result::Result<acp::NewSessionResponse, acp::Error> {
        let id = format!("session-{}", chrono::Utc::now().timestamp_millis());
        let _span = tracing::info_span!("acp_new_session", session = %id).entered();
        let wiki = req
            .meta
            .as_ref()
            .and_then(|m| m.get("wiki"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let session = AcpSession {
            id: id.clone(),
            label: None,
            wiki,
            created_at: chrono::Utc::now().timestamp() as u64,
            active_run: None,
        };
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(id.clone(), session);
        }
        tracing::info!(session = %id, "session created");
        Ok(acp::NewSessionResponse::new(id))
    }

    async fn load_session(
        &self,
        req: acp::LoadSessionRequest,
    ) -> std::result::Result<acp::LoadSessionResponse, acp::Error> {
        let exists = self
            .sessions
            .lock()
            .map(|s| s.contains_key(&*req.session_id.to_string()))
            .unwrap_or(false);
        if exists {
            Ok(acp::LoadSessionResponse::new())
        } else {
            Err(acp::Error::new(
                i32::from(acp::ErrorCode::InvalidParams),
                format!("session {} not found", req.session_id),
            ))
        }
    }

    async fn list_sessions(
        &self,
        _req: acp::ListSessionsRequest,
    ) -> std::result::Result<acp::ListSessionsResponse, acp::Error> {
        let cwd = self.session_cwd();
        let infos: Vec<acp::SessionInfo> = self
            .sessions
            .lock()
            .map(|sessions| {
                sessions
                    .values()
                    .map(|s| {
                        acp::SessionInfo::new(acp::SessionId::new(s.id.clone()), cwd.clone())
                            .title(s.label.clone())
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(acp::ListSessionsResponse::new(infos))
    }

    async fn prompt(
        &self,
        req: acp::PromptRequest,
    ) -> std::result::Result<acp::PromptResponse, acp::Error> {
        let text = Self::extract_prompt_text(&req);
        let (workflow, query) = Self::dispatch_workflow(&text);
        let session_id_str = req.session_id.to_string();
        let _span = tracing::info_span!(
            "acp_prompt",
            session = %session_id_str,
            workflow = %workflow,
        )
        .entered();

        let wiki_name = {
            let session_wiki = self
                .sessions
                .lock()
                .ok()
                .and_then(|s| s.get(&session_id_str).and_then(|sess| sess.wiki.clone()));
            self.resolve_wiki_name(session_wiki.as_deref())
        };

        // Mark active run
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(sess) = sessions.get_mut(&session_id_str) {
                sess.active_run = Some(format!("run-{}", chrono::Utc::now().timestamp_millis()));
            }
        }

        let query_text = if query.is_empty() { &text } else { query };

        match workflow {
            "research" => {
                return self
                    .run_research(&req.session_id, query_text, &wiki_name)
                    .await;
            }
            other => {
                // Unrecognized workflow — inform the user
                self.send_message(
                    &req.session_id,
                    &format!(
                        "Unknown workflow \"{other}\". Use `llm-wiki:research <query>` or ask a question directly."
                    ),
                )
                .await?;
            }
        }

        self.clear_active_run(&session_id_str);
        tracing::debug!(session = %session_id_str, workflow = %workflow, "prompt complete");
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn cancel(&self, req: acp::CancelNotification) -> std::result::Result<(), acp::Error> {
        let session_id = req.session_id.to_string();
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(sess) = sessions.get_mut(&session_id) {
                sess.active_run = None;
            }
        }
        Ok(())
    }
}

// ── serve_acp ─────────────────────────────────────────────────────────────────

pub async fn serve_acp(manager: Arc<WikiEngine>) -> Result<()> {
    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (tx, mut rx) = mpsc::unbounded_channel();
            let agent = WikiAgent::new(manager, tx);

            let (conn, handle_io) =
                acp::AgentSideConnection::new(agent, outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            tokio::task::spawn_local(async move {
                while let Some((notif, tx)) = rx.recv().await {
                    if let Err(e) = conn.session_notification(notif).await {
                        tracing::error!(error = %e, "ACP notification failed");
                        break;
                    }
                    tx.send(()).ok();
                }
            });

            handle_io.await
        })
        .await
        .map_err(|e| anyhow::anyhow!("ACP connection error: {e}"))
}
