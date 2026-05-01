use std::sync::atomic::Ordering;

use agent_client_protocol::schema::{SessionId, ToolCallStatus, ToolKind};
use agent_client_protocol::{Client, ConnectionTo};

use crate::engine::WikiEngine;
use crate::ops;

use super::helpers::{clear_active_run, get_cancelled, send_text, send_tool_call, send_tool_result, session_cwd};
use super::{Sessions, StepResult, make_tool_id};

pub fn step_ingest(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    session_id: &SessionId,
    path: &str,
    wiki_name: &str,
) -> StepResult {
    let tool_id = make_tool_id("ingest", "ingest");
    send_tool_call(
        cx,
        session_id,
        &tool_id,
        &format!("wiki_ingest: {path}"),
        ToolKind::Other,
    )?;

    let result = {
        let engine = manager
            .state
            .read()
            .map_err(|_| agent_client_protocol::schema::Error::internal_error())?;
        ops::ingest(&engine, manager, path, false, wiki_name)
    };

    match result {
        Ok(report) => {
            let commit_info = if report.commit.is_empty() {
                "no commit".to_string()
            } else {
                format!("commit {}", &report.commit[..8.min(report.commit.len())])
            };
            let summary = format!(
                "{} pages validated, {} unchanged, {} warnings — {commit_info}",
                report.pages_validated,
                report.unchanged_count,
                report.warnings.len(),
            );
            send_tool_result(
                cx,
                session_id,
                &tool_id,
                ToolCallStatus::Completed,
                &summary,
            )?;
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

pub fn run_ingest(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    sessions: &Sessions,
    session_id: &SessionId,
    query: &str,
    wiki_name: &str,
) -> StepResult {
    let cancelled = get_cancelled(sessions, &session_id.to_string());
    if cancelled.as_ref().map(|c| c.load(Ordering::Relaxed)).unwrap_or(false) {
        send_text(cx, session_id, "Cancelled.")?;
        clear_active_run(sessions, &session_id.to_string());
        return Ok(());
    }
    let path = if query.is_empty() {
        session_cwd(manager).to_string_lossy().into_owned()
    } else {
        query.to_string()
    };
    step_ingest(cx, manager, session_id, &path, wiki_name)?;
    clear_active_run(sessions, &session_id.to_string());
    Ok(())
}
