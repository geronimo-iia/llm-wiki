use agent_client_protocol::schema::{SessionId, ToolCallStatus, ToolKind};
use agent_client_protocol::{Client, ConnectionTo};

use crate::engine::WikiEngine;
use crate::ops;

use super::helpers::{clear_active_run, send_text, send_tool_call, send_tool_result};
use super::{make_tool_id, Sessions};

// ── Reusable workflow steps ───────────────────────────────────────────────────

pub fn step_search(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    session_id: &SessionId,
    workflow: &str,
    query: &str,
    wiki_name: &str,
    top_k: usize,
) -> std::result::Result<Vec<crate::search::PageRef>, agent_client_protocol::schema::Error> {
    let tool_id = make_tool_id(workflow, "search");
    send_tool_call(
        cx,
        session_id,
        &tool_id,
        &format!("wiki_search: {query}"),
        ToolKind::Search,
    )?;

    let results = {
        let engine = manager
            .state
            .read()
            .map_err(|_| agent_client_protocol::schema::Error::internal_error())?;
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
            send_tool_result(
                cx,
                session_id,
                &tool_id,
                ToolCallStatus::Completed,
                &format!("{} results", results.len()),
            )?;
            Ok(results)
        }
        Err(e) => {
            send_tool_result(
                cx,
                session_id,
                &tool_id,
                ToolCallStatus::Failed,
                &format!("{e}"),
            )?;
            Ok(Vec::new())
        }
    }
}

pub fn step_read(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    session_id: &SessionId,
    workflow: &str,
    slug: &str,
    wiki_name: &str,
) -> std::result::Result<(), agent_client_protocol::schema::Error> {
    let tool_id = make_tool_id(workflow, "read");
    send_tool_call(
        cx,
        session_id,
        &tool_id,
        &format!("wiki_content_read: {slug}"),
        ToolKind::Read,
    )?;

    let result = {
        let engine = manager
            .state
            .read()
            .map_err(|_| agent_client_protocol::schema::Error::internal_error())?;
        ops::content_read(&engine, slug, Some(wiki_name), false, false)
    };

    match result {
        Ok(_) => send_tool_result(cx, session_id, &tool_id, ToolCallStatus::Completed, ""),
        Err(e) => send_tool_result(
            cx,
            session_id,
            &tool_id,
            ToolCallStatus::Failed,
            &format!("{e}"),
        ),
    }
}

pub fn step_report_results(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    results: &[crate::search::PageRef],
    wiki_name: &str,
) -> std::result::Result<(), agent_client_protocol::schema::Error> {
    if results.is_empty() {
        return Ok(());
    }
    let hits: Vec<String> = results
        .iter()
        .take(5)
        .map(|r| format!("- {} (score: {:.2})", r.uri, r.score))
        .collect();
    send_text(
        cx,
        session_id,
        &format!(
            "Based on {} pages in \"{wiki_name}\":\n{}",
            results.len(),
            hits.join("\n")
        ),
    )
}

// ── Workflows ─────────────────────────────────────────────────────────────────

pub fn run_research(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    sessions: &Sessions,
    session_id: &SessionId,
    query: &str,
    wiki_name: &str,
) -> std::result::Result<(), agent_client_protocol::schema::Error> {
    send_text(cx, session_id, &format!("Searching for: {query}..."))?;

    let results = step_search(cx, manager, session_id, "research", query, wiki_name, 5)?;

    if results.is_empty() {
        send_text(
            cx,
            session_id,
            &format!("No results found for \"{query}\" in wiki \"{wiki_name}\"."),
        )?;
    } else {
        step_read(
            cx,
            manager,
            session_id,
            "research",
            &results[0].slug,
            wiki_name,
        )?;
        step_report_results(cx, session_id, &results, wiki_name)?;
    }

    clear_active_run(sessions, &session_id.to_string());
    Ok(())
}
