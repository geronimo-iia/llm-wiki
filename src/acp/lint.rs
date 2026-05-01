use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use agent_client_protocol::schema::{SessionId, ToolCallStatus, ToolKind};
use agent_client_protocol::{Client, ConnectionTo};

use crate::engine::WikiEngine;
use crate::ops;

use super::helpers::{clear_active_run, get_cancelled, send_text, send_tool_call, send_tool_result};
use super::{Sessions, StepResult, make_tool_id};

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
            .map_err(|_| agent_client_protocol::schema::Error::internal_error())?;
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
                if cancelled.as_ref().map(|c| c.load(Ordering::Relaxed)).unwrap_or(false) {
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
