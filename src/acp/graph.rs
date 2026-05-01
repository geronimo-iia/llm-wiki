use agent_client_protocol::schema::{SessionId, ToolCallStatus, ToolKind};
use agent_client_protocol::{Client, ConnectionTo};

use crate::engine::WikiEngine;
use crate::ops::{self, GraphParams};

use super::helpers::{clear_active_run, send_text, send_tool_call, send_tool_result};
use super::{Sessions, StepResult, make_tool_id};

pub fn step_graph(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    session_id: &SessionId,
    root: Option<&str>,
    wiki_name: &str,
) -> StepResult {
    let tool_id = make_tool_id("graph", "graph");
    let label = root
        .map(|r| format!("wiki_graph root={r}"))
        .unwrap_or_else(|| "wiki_graph".to_string());

    send_tool_call(cx, session_id, &tool_id, &label, ToolKind::Other)?;

    let result = {
        let engine = manager
            .state
            .read()
            .map_err(|_| agent_client_protocol::schema::Error::internal_error())?;
        ops::graph_build(
            &engine,
            wiki_name,
            &GraphParams {
                format: Some("llms"),
                root: root.map(str::to_string),
                depth: None,
                type_filter: None,
                relation: None,
                output: None,
                cross_wiki: false,
            },
        )
    };

    match result {
        Ok(gr) => {
            let summary = format!(
                "Graph: {} nodes, {} edges",
                gr.report.nodes, gr.report.edges
            );
            send_tool_result(
                cx,
                session_id,
                &tool_id,
                ToolCallStatus::Completed,
                &summary,
            )?;
            send_text(cx, session_id, &gr.rendered)?;
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

pub fn run_graph(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    sessions: &Sessions,
    session_id: &SessionId,
    query: &str,
    wiki_name: &str,
) -> StepResult {
    let root = (!query.is_empty()).then_some(query);
    step_graph(cx, manager, session_id, root, wiki_name)?;
    clear_active_run(sessions, &session_id.to_string());
    Ok(())
}
