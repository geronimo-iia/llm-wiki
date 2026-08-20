#![allow(unreachable_pub)]
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agent_client_protocol::schema::v1::{SessionId, ToolCallStatus, ToolKind};
use agent_client_protocol::{Client, ConnectionTo};

use crate::engine::WikiEngine;
use crate::ops;

use super::helpers::{
    clear_active_run, get_cancelled, send_text, send_tool_call, send_tool_result,
};
use super::{Sessions, StepResult, make_tool_id};

/// Execute the lint step: run the requested lint rules and format findings.
pub fn step_lint(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    session_id: &SessionId,
    wiki_name: &str,
    rules: Option<&str>,
    cancelled: Option<Arc<AtomicBool>>,
) -> StepResult {
    let tool_id = make_tool_id("lint", "lint");
    let label = rules
        .filter(|r| !r.is_empty())
        .map(|r| format!("wiki_lint rules={r}"))
        .unwrap_or_else(|| "wiki_lint".to_string());

    send_tool_call(cx, session_id, &tool_id, &label, ToolKind::Other)?;

    let result = {
        let engine = manager
            .state
            .read()
            .map_err(|_| agent_client_protocol::schema::v1::Error::internal_error())?;
        ops::run_lint(&engine, wiki_name, rules, None)
    };

    match result {
        Ok(report) => {
            let summary = format!(
                "{} findings ({} errors, {} warnings)",
                report.total, report.errors, report.warnings
            );
            send_tool_result(
                cx,
                session_id,
                &tool_id,
                ToolCallStatus::Completed,
                &summary,
            )?;
            for f in &report.findings {
                if cancelled
                    .as_ref()
                    .map(|c| c.load(Ordering::Relaxed))
                    .unwrap_or(false)
                {
                    send_text(cx, session_id, "Cancelled.")?;
                    return Ok(());
                }
                send_text(
                    cx,
                    session_id,
                    &format!("[{}] {}: {}", f.severity, f.slug, f.message),
                )?;
            }
            Ok(())
        }
        Err(e) => {
            send_tool_result(
                cx,
                session_id,
                &tool_id,
                ToolCallStatus::Failed,
                &format!("{e}"),
            )?;
            Ok(())
        }
    }
}

/// Run the full lint ACP workflow from params to final text response.
pub fn run_lint(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    sessions: &Sessions,
    session_id: &SessionId,
    query: &str,
    wiki_name: &str,
) -> StepResult {
    let cancelled = get_cancelled(sessions, &session_id.to_string());
    let rules = (!query.is_empty()).then_some(query);
    step_lint(cx, manager, session_id, wiki_name, rules, cancelled)?;
    clear_active_run(sessions, &session_id.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use agent_client_protocol::schema::v1::SessionId;
    use agent_client_protocol::{Agent, Builder, Channel};
    use parking_lot::Mutex;

    use crate::acp::{AcpSession, Sessions};

    use super::{run_lint, step_lint};

    fn make_engine() -> (crate::engine::WikiEngine, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("llm-wiki.toml");
        std::fs::write(&config_path, "").unwrap();
        let engine = crate::engine::WikiEngine::build(&config_path).unwrap();
        (engine, dir)
    }

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
                active_run: None,
                cancelled: Arc::new(AtomicBool::new(false)),
            },
        );
    }

    #[tokio::test]
    async fn step_lint_no_rules_succeeds() {
        let (engine, _dir) = make_engine();
        let session_id = SessionId::new("sess");
        let (chan_a, _chan_b) = Channel::duplex();
        Builder::<Agent>::new(Agent)
            .connect_with(chan_a, async |cx| {
                let result = step_lint(&cx, &engine, &session_id, "no-such-wiki", None, None);
                assert!(result.is_ok());
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn step_lint_with_rules_succeeds() {
        let (engine, _dir) = make_engine();
        let session_id = SessionId::new("sess");
        let (chan_a, _chan_b) = Channel::duplex();
        Builder::<Agent>::new(Agent)
            .connect_with(chan_a, async |cx| {
                let result =
                    step_lint(&cx, &engine, &session_id, "no-such-wiki", Some("orphan"), None);
                assert!(result.is_ok());
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_lint_empty_query_passes_none_rules() {
        let (engine, _dir) = make_engine();
        let sessions = make_sessions();
        insert_session(&sessions, "s1");
        let session_id = SessionId::new("s1");
        let (chan_a, _chan_b) = Channel::duplex();
        Builder::<Agent>::new(Agent)
            .connect_with(chan_a, async |cx| {
                let result = run_lint(&cx, &engine, &sessions, &session_id, "", "no-such-wiki");
                assert!(result.is_ok());
                assert!(sessions.lock().get("s1").unwrap().active_run.is_none());
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_lint_non_empty_query_passes_rules() {
        let (engine, _dir) = make_engine();
        let sessions = make_sessions();
        insert_session(&sessions, "s2");
        let session_id = SessionId::new("s2");
        let (chan_a, _chan_b) = Channel::duplex();
        Builder::<Agent>::new(Agent)
            .connect_with(chan_a, async |cx| {
                let result =
                    run_lint(&cx, &engine, &sessions, &session_id, "stale,orphan", "no-such-wiki");
                assert!(result.is_ok());
                Ok(())
            })
            .await
            .unwrap();
    }
}
