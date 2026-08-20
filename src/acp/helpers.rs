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
    let engine = manager.state.read().unwrap_or_else(|e| e.into_inner());
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
    let engine = manager.state.read().unwrap_or_else(|e| e.into_inner());
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use parking_lot::Mutex;

    use crate::acp::{AcpSession, Sessions};

    use super::{clear_active_run, get_cancelled};

    fn make_sessions() -> Sessions {
        Arc::new(Mutex::new(HashMap::new()))
    }

    fn insert_session(sessions: &Sessions, id: &str) {
        sessions.lock().insert(
            id.to_string(),
            AcpSession {
                id: id.to_string(),
                label: None,
                wiki: None,
                created_at: 0,
                active_run: Some("run-1".to_string()),
                cancelled: Arc::new(AtomicBool::new(false)),
            },
        );
    }

    #[test]
    fn get_cancelled_unknown_session_returns_none() {
        let sessions = make_sessions();
        assert!(get_cancelled(&sessions, "ghost").is_none());
    }

    #[test]
    fn get_cancelled_known_session_returns_flag() {
        let sessions = make_sessions();
        insert_session(&sessions, "s1");
        let flag = get_cancelled(&sessions, "s1").expect("flag must exist");
        assert!(!flag.load(Ordering::Relaxed), "flag starts false");
    }

    #[test]
    fn get_cancelled_flag_is_shared_with_session() {
        let sessions = make_sessions();
        insert_session(&sessions, "s2");
        let flag = get_cancelled(&sessions, "s2").unwrap();
        sessions
            .lock()
            .get("s2")
            .unwrap()
            .cancelled
            .store(true, Ordering::Relaxed);
        assert!(flag.load(Ordering::Relaxed), "shared Arc — mutation visible");
    }

    #[test]
    fn clear_active_run_unknown_session_is_noop() {
        let sessions = make_sessions();
        clear_active_run(&sessions, "nobody");
    }

    #[test]
    fn clear_active_run_clears_active_run_field() {
        let sessions = make_sessions();
        insert_session(&sessions, "s3");
        assert!(sessions.lock().get("s3").unwrap().active_run.is_some());
        clear_active_run(&sessions, "s3");
        assert!(sessions.lock().get("s3").unwrap().active_run.is_none());
    }

    #[test]
    fn clear_active_run_does_not_remove_session() {
        let sessions = make_sessions();
        insert_session(&sessions, "s4");
        clear_active_run(&sessions, "s4");
        assert!(sessions.lock().contains_key("s4"));
    }
}
