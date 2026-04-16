use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{self as acp, Client as _};
use anyhow::Result;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::config::{GlobalConfig, WikiEntry};
use crate::server::INSTRUCTIONS;
use crate::spaces;

pub struct AcpSession {
    pub id: String,
    pub label: Option<String>,
    pub wiki: Option<String>,
    pub created_at: u64,
    pub active_run: Option<String>,
}

pub struct WikiAgent {
    pub spaces: Arc<Vec<WikiEntry>>,
    pub global: Arc<GlobalConfig>,
    sessions: Mutex<HashMap<String, AcpSession>>,
    update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
}

impl WikiAgent {
    pub fn new(
        global: Arc<GlobalConfig>,
        update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        let wikis = spaces::load_all(&global);
        Self {
            spaces: Arc::new(wikis),
            global,
            sessions: Mutex::new(HashMap::new()),
            update_tx,
        }
    }

    fn resolve_wiki(&self, name: Option<&str>) -> Option<&WikiEntry> {
        match name {
            Some(n) => self.spaces.iter().find(|w| w.name == n),
            None => self
                .spaces
                .iter()
                .find(|w| w.name == self.global.global.default_wiki),
        }
    }

    fn dispatch_workflow(prompt: &str) -> &'static str {
        let lower = prompt.to_lowercase();
        if lower.contains("ingest")
            || lower.contains("add")
            || lower.contains('/')
            || lower.contains('\\')
        {
            "ingest"
        } else if lower.contains("lint") || lower.contains("orphan") || lower.contains("stub") {
            "lint"
        } else if lower.contains("crystallize")
            || lower.contains("distil")
            || lower.contains("capture")
        {
            "crystallize"
        } else {
            "research"
        }
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

    fn session_cwd(&self) -> PathBuf {
        self.resolve_wiki(None)
            .map(|e| PathBuf::from(&e.path))
            .unwrap_or_else(|| PathBuf::from("."))
    }

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

    async fn run_research(
        &self,
        session_id: &acp::SessionId,
        query: &str,
        wiki_entry: Option<&WikiEntry>,
        wiki_name: &str,
    ) -> std::result::Result<acp::PromptResponse, acp::Error> {
        let entry = match wiki_entry {
            Some(e) => e,
            None => {
                self.send_message(session_id, "No wiki configured. Register a wiki with `wiki spaces add`.")
                    .await?;
                self.clear_active_run(&session_id.to_string());
                return Ok(acp::PromptResponse::new(acp::StopReason::EndTurn));
            }
        };

        // Step 1: announce search
        self.send_message(session_id, &format!("Searching for: {query}..."))
            .await?;

        // Step 2: tool call — search
        let search_id = Self::make_tool_id("research", "search");
        self.send_tool_call(
            session_id,
            &search_id,
            &format!("wiki_search: {query}"),
            acp::ToolKind::Search,
        )
        .await?;

        // Step 3: execute search
        let index_path = crate::server::WikiServer::index_path_for(&entry.name);
        let opts = crate::search::SearchOptions::default();
        let results = crate::search::search(query, &opts, &index_path, &entry.name, None);

        match results {
            Ok(results) if !results.is_empty() => {
                self.send_tool_result(
                    session_id,
                    &search_id,
                    acp::ToolCallStatus::Completed,
                    &format!("{} results", results.len()),
                )
                .await?;

                // Step 4: read top result
                let top = &results[0];
                let read_id = Self::make_tool_id("research", "read");
                self.send_tool_call(
                    session_id,
                    &read_id,
                    &format!("wiki_read: {}", top.slug),
                    acp::ToolKind::Read,
                )
                .await?;

                let wiki_root = PathBuf::from(&entry.path).join("wiki");
                let read_result =
                    crate::markdown::read_page(&top.slug, &wiki_root, false);
                match read_result {
                    Ok(_) => {
                        self.send_tool_result(
                            session_id,
                            &read_id,
                            acp::ToolCallStatus::Completed,
                            "",
                        )
                        .await?;
                    }
                    Err(e) => {
                        self.send_tool_result(
                            session_id,
                            &read_id,
                            acp::ToolCallStatus::Failed,
                            &format!("{e}"),
                        )
                        .await?;
                    }
                }

                // Step 5: summary message
                let hits: Vec<String> = results
                    .iter()
                    .take(5)
                    .map(|r| format!("- {} (score: {:.2})", r.uri, r.score))
                    .collect();
                self.send_message(
                    session_id,
                    &format!(
                        "Based on {} pages in \"{}\":\n{}",
                        results.len(),
                        wiki_name,
                        hits.join("\n")
                    ),
                )
                .await?;
            }
            Ok(_) => {
                self.send_tool_result(
                    session_id,
                    &search_id,
                    acp::ToolCallStatus::Completed,
                    "0 results",
                )
                .await?;
                self.send_message(
                    session_id,
                    &format!("No results found for \"{query}\" in wiki \"{wiki_name}\"."),
                )
                .await?;
            }
            Err(e) => {
                self.send_tool_result(
                    session_id,
                    &search_id,
                    acp::ToolCallStatus::Failed,
                    &format!("{e}"),
                )
                .await?;
                self.send_message(session_id, &format!("Search failed: {e}"))
                    .await?;
            }
        }

        self.clear_active_run(&session_id.to_string());
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn run_lint(
        &self,
        session_id: &acp::SessionId,
        wiki_entry: Option<&WikiEntry>,
        wiki_name: &str,
    ) -> std::result::Result<acp::PromptResponse, acp::Error> {
        let entry = match wiki_entry {
            Some(e) => e,
            None => {
                self.send_message(session_id, "No wiki found for lint workflow.")
                    .await?;
                self.clear_active_run(&session_id.to_string());
                return Ok(acp::PromptResponse::new(acp::StopReason::EndTurn));
            }
        };

        // Step 1: announce
        self.send_message(session_id, "Running lint...").await?;

        // Step 2: tool call
        let lint_id = Self::make_tool_id("lint", "run");
        self.send_tool_call(
            session_id,
            &lint_id,
            &format!("wiki_lint: {wiki_name}"),
            acp::ToolKind::Execute,
        )
        .await?;

        // Step 3: execute
        let wiki_root = PathBuf::from(&entry.path).join("wiki");
        let wiki_cfg =
            crate::config::load_wiki(&PathBuf::from(&entry.path)).unwrap_or_default();
        let resolved = crate::config::resolve(&self.global, &wiki_cfg);

        match crate::lint::lint(&wiki_root, &resolved, &entry.name) {
            Ok(report) => {
                let summary = format!(
                    "{} orphans, {} stubs, {} empty sections",
                    report.orphans.len(),
                    report.missing_stubs.len(),
                    report.empty_sections.len(),
                );
                self.send_tool_result(
                    session_id,
                    &lint_id,
                    acp::ToolCallStatus::Completed,
                    &summary,
                )
                .await?;
                self.send_message(
                    session_id,
                    &format!(
                        "Lint report for \"{wiki_name}\": {summary}, \
                         {} missing connections, {} untyped sources.",
                        report.missing_connections.len(),
                        report.untyped_sources.len(),
                    ),
                )
                .await?;
            }
            Err(e) => {
                self.send_tool_result(
                    session_id,
                    &lint_id,
                    acp::ToolCallStatus::Failed,
                    &format!("{e}"),
                )
                .await?;
                self.send_message(session_id, &format!("Lint failed: {e}"))
                    .await?;
            }
        }

        self.clear_active_run(&session_id.to_string());
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for WikiAgent {
    async fn initialize(
        &self,
        _args: acp::InitializeRequest,
    ) -> std::result::Result<acp::InitializeResponse, acp::Error> {
        let mut meta = serde_json::Map::new();
        meta.insert(
            "system".to_string(),
            serde_json::Value::String(INSTRUCTIONS.to_string()),
        );
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
            ))
            .meta(meta))
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
        let workflow = Self::dispatch_workflow(&text);
        let session_id_str = req.session_id.to_string();
        let _span = tracing::info_span!(
            "acp_prompt",
            session = %session_id_str,
            workflow = %workflow,
        )
        .entered();

        let wiki_name = self
            .sessions
            .lock()
            .ok()
            .and_then(|s| s.get(&session_id_str).and_then(|sess| sess.wiki.clone()));
        let wiki_entry = self.resolve_wiki(wiki_name.as_deref()).cloned();

        // Mark active run
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(sess) = sessions.get_mut(&session_id_str) {
                sess.active_run = Some(format!("run-{}", chrono::Utc::now().timestamp_millis()));
            }
        }

        let name = wiki_entry
            .as_ref()
            .map(|e| e.name.as_str())
            .unwrap_or("default");

        let result = match workflow {
            "research" => {
                return self
                    .run_research(&req.session_id, &text, wiki_entry.as_ref(), name)
                    .await;
            }
            "lint" => {
                return self
                    .run_lint(&req.session_id, wiki_entry.as_ref(), name)
                    .await;
            }
            _ => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match workflow {
                    "ingest" => {
                        format!("Ingest workflow triggered for wiki \"{name}\". Prompt: {text}")
                    }
                    "crystallize" => {
                        format!(
                            "Crystallize workflow triggered for wiki \"{name}\". Prompt: {text}"
                        )
                    }
                    _ => unreachable!(),
                }
            })),
        };
        let result = match result {
            Ok(r) => r,
            Err(_) => {
                tracing::error!(session = %session_id_str, workflow = %workflow, "workflow panicked");
                "Internal error: workflow panicked".to_string()
            }
        };

        self.send_message(&req.session_id, &result).await?;

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

pub async fn serve_acp(global: Arc<GlobalConfig>) -> Result<()> {
    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (tx, mut rx) = mpsc::unbounded_channel();
            let agent = WikiAgent::new(global, tx);

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
