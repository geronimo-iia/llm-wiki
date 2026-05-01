use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol::schema::{
    AgentCapabilities, CancelNotification, InitializeRequest, InitializeResponse,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PromptCapabilities, PromptRequest, PromptResponse,
    SessionCapabilities, SessionId, SessionInfo, SessionListCapabilities, StopReason,
};
use agent_client_protocol::{
    Agent, ByteStreams, Client, ConnectionTo, Dispatch, on_receive_dispatch,
    on_receive_notification, on_receive_request,
};
use anyhow::Result;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::engine::WikiEngine;

use super::graph::run_graph;
use super::helpers::{clear_active_run, resolve_wiki_name, send_text, session_cwd};
use super::ingest::run_ingest;
use super::lint::run_lint;
use super::research::run_research;
use super::{AcpSession, Sessions, dispatch_workflow, extract_prompt_text};

/// Start the ACP (Agent Client Protocol) server on stdio.
pub async fn serve_acp(manager: Arc<WikiEngine>) -> Result<()> {
    let sessions: Sessions = Arc::new(std::sync::Mutex::new(HashMap::new()));

    Agent
        .builder()
        .name("llm-wiki")
        // ── Initialize ───────────────────────────────────────────────
        .on_receive_request(
            async move |req: InitializeRequest, responder, _cx| {
                responder.respond(
                    InitializeResponse::new(req.protocol_version)
                        .agent_capabilities(
                            AgentCapabilities::new()
                                .prompt_capabilities(PromptCapabilities::new())
                                .session_capabilities(
                                    SessionCapabilities::new()
                                        .list(SessionListCapabilities::new()),
                                ),
                        )
                        .agent_info(agent_client_protocol::schema::Implementation::new(
                            "llm-wiki",
                            env!("CARGO_PKG_VERSION"),
                        )),
                )
            },
            on_receive_request!(),
        )
        // ── NewSession ───────────────────────────────────────────────
        .on_receive_request(
            {
                let sessions = sessions.clone();
                async move |req: NewSessionRequest, responder, _cx| {
                    let id = format!("session-{}", chrono::Utc::now().timestamp_millis());
                    let _span =
                        tracing::info_span!("acp_new_session", session = %id).entered();
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
                    sessions.lock().unwrap().insert(id.clone(), session);
                    tracing::info!(session = %id, "session created");
                    responder.respond(NewSessionResponse::new(SessionId::new(id)))
                }
            },
            on_receive_request!(),
        )
        // ── LoadSession ──────────────────────────────────────────────
        .on_receive_request(
            {
                let sessions = sessions.clone();
                async move |req: LoadSessionRequest, responder, _cx| {
                    let exists = sessions
                        .lock()
                        .map(|s| s.contains_key(&req.session_id.to_string()))
                        .unwrap_or(false);
                    if exists {
                        responder.respond(LoadSessionResponse::new())
                    } else {
                        responder.respond_with_error(
                            agent_client_protocol::schema::Error::new(
                                i32::from(agent_client_protocol::schema::ErrorCode::InvalidParams),
                                format!("session {} not found", req.session_id),
                            ),
                        )
                    }
                }
            },
            on_receive_request!(),
        )
        // ── ListSessions ─────────────────────────────────────────────
        .on_receive_request(
            {
                let mgr = manager.clone();
                let sessions = sessions.clone();
                async move |_req: ListSessionsRequest, responder, _cx| {
                    let cwd = session_cwd(&mgr);
                    let infos: Vec<SessionInfo> = sessions
                        .lock()
                        .map(|s| {
                            s.values()
                                .map(|sess| {
                                    SessionInfo::new(
                                        SessionId::new(sess.id.clone()),
                                        cwd.clone(),
                                    )
                                    .title(sess.label.clone())
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    responder.respond(ListSessionsResponse::new(infos))
                }
            },
            on_receive_request!(),
        )
        // ── Prompt ───────────────────────────────────────────────────
        .on_receive_request(
            {
                let mgr = manager.clone();
                let sessions = sessions.clone();
                async move |req: PromptRequest, responder, cx: ConnectionTo<Client>| {
                    let text = extract_prompt_text(&req);
                    let (workflow, query) = dispatch_workflow(&text);
                    let session_id_str = req.session_id.to_string();
                    let _span = tracing::info_span!(
                        "acp_prompt",
                        session = %session_id_str,
                        workflow = %workflow,
                    )
                    .entered();

                    let wiki_name = resolve_wiki_name(&mgr, &sessions, &req.session_id);

                    // Mark active run
                    if let Ok(mut s) = sessions.lock()
                        && let Some(sess) = s.get_mut(&session_id_str)
                    {
                        sess.active_run =
                            Some(format!("run-{}", chrono::Utc::now().timestamp_millis()));
                    }

                    let query_text = if query.is_empty() { &text } else { query };

                    match workflow {
                        "research" => {
                            run_research(&cx, &mgr, &sessions, &req.session_id, query_text, &wiki_name)?;
                        }
                        "lint" => {
                            run_lint(&cx, &mgr, &sessions, &req.session_id, query_text, &wiki_name)?;
                        }
                        "graph" => {
                            run_graph(&cx, &mgr, &sessions, &req.session_id, query_text, &wiki_name)?;
                        }
                        "ingest" => {
                            run_ingest(&cx, &mgr, &sessions, &req.session_id, query_text, &wiki_name)?;
                        }
                        "use" => {
                            use super::research::step_read;
                            if query_text.is_empty() {
                                send_text(&cx, &req.session_id, "Usage: `llm-wiki:use <slug>`")?;
                            } else {
                                step_read(&cx, &mgr, &req.session_id, "use", query_text, &wiki_name, true)?;
                            }
                            clear_active_run(&sessions, &session_id_str);
                        }
                        _ => {
                            let msg = if workflow != "help" {
                                format!(
                                    "Unknown workflow \"{workflow}\". Available workflows:\n\
                                     • `llm-wiki:research <query>` — search + read top result\n\
                                     • `llm-wiki:lint [rules]`      — run lint rules (comma-separated or all)\n\
                                     • `llm-wiki:graph [root-slug]` — render concept graph\n\
                                     • `llm-wiki:ingest [path]`     — ingest path (default: cwd)\n\
                                     • `llm-wiki:use <slug>`        — read full page content\n\
                                     • `llm-wiki:help`              — this message\n\
                                     • (bare prompt)                — research workflow"
                                )
                            } else {
                                "Available workflows:\n\
                                 • `llm-wiki:research <query>` — search + read top result\n\
                                 • `llm-wiki:lint [rules]`      — run lint rules (comma-separated or all)\n\
                                 • `llm-wiki:graph [root-slug]` — render concept graph\n\
                                 • `llm-wiki:ingest [path]`     — ingest path (default: cwd)\n\
                                 • `llm-wiki:use <slug>`        — read full page content\n\
                                 • `llm-wiki:help`              — this message\n\
                                 • (bare prompt)                — research workflow"
                                    .to_string()
                            };
                            send_text(&cx, &req.session_id, &msg)?;
                            clear_active_run(&sessions, &session_id_str);
                        }
                    }
                    tracing::debug!(session = %session_id_str, workflow = %workflow, "prompt complete");
                    responder.respond(PromptResponse::new(StopReason::EndTurn))
                }
            },
            on_receive_request!(),
        )
        // ── Cancel ───────────────────────────────────────────────────
        .on_receive_notification(
            {
                let sessions = sessions.clone();
                async move |notif: CancelNotification, _cx| {
                    clear_active_run(&sessions, &notif.session_id.to_string());
                    Ok(())
                }
            },
            on_receive_notification!(),
        )
        // ── Catch-all ────────────────────────────────────────────────
        .on_receive_dispatch(
            async move |msg: Dispatch, cx: ConnectionTo<Client>| {
                msg.respond_with_error(
                    agent_client_protocol::util::internal_error("not supported"),
                    cx,
                )
            },
            on_receive_dispatch!(),
        )
        .connect_to(ByteStreams::new(
            tokio::io::stdout().compat_write(),
            tokio::io::stdin().compat(),
        ))
        .await
        .map_err(|e| anyhow::anyhow!("ACP error: {e}"))
}
