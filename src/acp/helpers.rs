#![allow(unreachable_pub)]
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use agent_client_protocol::Client;
use agent_client_protocol::ConnectionTo;
use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, SessionId, SessionNotification, SessionUpdate, TextContent,
    ToolCall, ToolCallId, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};

use crate::engine::WikiEngine;

use super::Sessions;

// ── Streaming helpers ─────────────────────────────────────────────────────────

/// Send a text content block to the ACP session response stream.
pub fn send_text(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    text: &str,
) -> std::result::Result<(), agent_client_protocol::schema::v1::Error> {
    cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            text,
        )))),
    ))
}

/// Send a tool-use content block (request) to the ACP session response stream.
pub fn send_tool_call(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    id: &str,
    title: &str,
    kind: ToolKind,
) -> std::result::Result<(), agent_client_protocol::schema::v1::Error> {
    cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::ToolCall(
            ToolCall::new(ToolCallId::new(id), title)
                .kind(kind)
                .status(ToolCallStatus::InProgress),
        ),
    ))
}

/// Send a tool-result content block to the ACP session response stream.
pub fn send_tool_result(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    id: &str,
    status: ToolCallStatus,
    content: &str,
) -> std::result::Result<(), agent_client_protocol::schema::v1::Error> {
    cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
            ToolCallId::new(id),
            ToolCallUpdateFields::new()
                .status(status)
                .content(vec![content.into()]),
        )),
    ))
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Resolve the target wiki name from ACP params, falling back to the engine default.
pub fn resolve_wiki_name(
    manager: &WikiEngine,
    sessions: &Sessions,
    session_id: &SessionId,
) -> String {
    let session_wiki = {
        let s = sessions.lock();
        s.get(&session_id.to_string())
            .and_then(|sess| sess.wiki.clone())
    };
    let engine = manager.state.read().expect("engine lock poisoned");
    engine
        .resolve_wiki_name(session_wiki.as_deref())
        .map(|s| s.to_string())
        .unwrap_or_else(|e| {
            tracing::warn!("ACP: {e}");
            String::new()
        })
}

/// Return the working directory for an ACP session (repo root of the default wiki).
pub fn session_cwd(manager: &WikiEngine) -> PathBuf {
    let engine = manager.state.read().expect("engine lock poisoned");
    engine
        .default_wiki_name()
        .and_then(|name| engine.space(name).ok())
        .map(|s| s.repo_root.clone())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Load the cancellation flag for a session. Returns None if session not found.
pub fn get_cancelled(sessions: &Sessions, session_id: &str) -> Option<Arc<AtomicBool>> {
    sessions.lock().get(session_id)?.cancelled.clone().into()
}

/// Clear the active-run flag for a session after a workflow completes or is cancelled.
pub fn clear_active_run(sessions: &Sessions, session_id: &str) {
    let mut s = sessions.lock();
    if let Some(sess) = s.get_mut(session_id) {
        sess.active_run = None;
    }
}
