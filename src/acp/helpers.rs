use std::path::PathBuf;

use agent_client_protocol::schema::{
    ContentBlock, ContentChunk, SessionId, SessionNotification, SessionUpdate, TextContent,
    ToolCall, ToolCallId, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
use agent_client_protocol::Client;
use agent_client_protocol::ConnectionTo;

use crate::engine::WikiEngine;

use super::Sessions;

// ── Streaming helpers ─────────────────────────────────────────────────────────

pub fn send_text(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    text: &str,
) -> std::result::Result<(), agent_client_protocol::schema::Error> {
    cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            text,
        )))),
    ))
}

pub fn send_tool_call(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    id: &str,
    title: &str,
    kind: ToolKind,
) -> std::result::Result<(), agent_client_protocol::schema::Error> {
    cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::ToolCall(
            ToolCall::new(ToolCallId::new(id), title)
                .kind(kind)
                .status(ToolCallStatus::InProgress),
        ),
    ))
}

pub fn send_tool_result(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    id: &str,
    status: ToolCallStatus,
    content: &str,
) -> std::result::Result<(), agent_client_protocol::schema::Error> {
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

pub fn resolve_wiki_name(
    manager: &WikiEngine,
    sessions: &Sessions,
    session_id: &SessionId,
) -> String {
    let session_wiki = sessions.lock().ok().and_then(|s| {
        s.get(&session_id.to_string())
            .and_then(|sess| sess.wiki.clone())
    });
    let engine = manager.state.read().expect("engine lock poisoned");
    engine
        .resolve_wiki_name(session_wiki.as_deref())
        .to_string()
}

pub fn session_cwd(manager: &WikiEngine) -> PathBuf {
    let engine = manager.state.read().expect("engine lock poisoned");
    let name = engine.default_wiki_name();
    engine
        .space(name)
        .map(|s| s.repo_root.clone())
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn clear_active_run(sessions: &Sessions, session_id: &str) {
    if let Ok(mut s) = sessions.lock() {
        if let Some(sess) = s.get_mut(session_id) {
            sess.active_run = None;
        }
    }
}
