#![allow(unreachable_pub)]
use std::sync::atomic::Ordering;

use agent_client_protocol::schema::v1::{SessionId, ToolCallStatus, ToolKind};
use agent_client_protocol::{Client, ConnectionTo};

use crate::engine::WikiEngine;
use crate::ops;

use super::helpers::{
    clear_active_run, get_cancelled, send_text, send_tool_call, send_tool_result,
};
use super::{Sessions, make_tool_id};

// ── Reusable workflow steps ───────────────────────────────────────────────────

/// Execute the search step of a research workflow: query the index and return ranked refs.
pub fn step_search(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    session_id: &SessionId,
    workflow: &str,
    query: &str,
    wiki_name: &str,
    top_k: usize,
) -> std::result::Result<Vec<crate::search::PageRef>, agent_client_protocol::schema::v1::Error> {
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
            .map_err(|_| agent_client_protocol::schema::v1::Error::internal_error())?;
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
        Ok(sr) => {
            send_tool_result(
                cx,
                session_id,
                &tool_id,
                ToolCallStatus::Completed,
                &format!("{} results", sr.results.len()),
            )?;
            Ok(sr.results)
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

/// Execute the read step: fetch full page content for a resolved slug.
pub fn step_read(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    session_id: &SessionId,
    workflow: &str,
    slug: &str,
    wiki_name: &str,
    stream_content: bool,
) -> std::result::Result<(), agent_client_protocol::schema::v1::Error> {
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
            .map_err(|_| agent_client_protocol::schema::v1::Error::internal_error())?;
        ops::content_read(&engine, slug, Some(wiki_name), false, false)
    };

    match result {
        Ok(crate::ops::ContentReadResult::Page(body)) => {
            send_tool_result(cx, session_id, &tool_id, ToolCallStatus::Completed, "")?;
            if stream_content {
                send_text(cx, session_id, &body)?;
            }
            Ok(())
        }
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

/// Execute the report step: format accumulated research results as a final response.
pub fn step_report_results(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    results: &[crate::search::PageRef],
    wiki_name: &str,
) -> std::result::Result<(), agent_client_protocol::schema::v1::Error> {
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

/// Run the full research ACP workflow from params to final text response.
pub fn run_research(
    cx: &ConnectionTo<Client>,
    manager: &WikiEngine,
    sessions: &Sessions,
    session_id: &SessionId,
    query: &str,
    wiki_name: &str,
) -> std::result::Result<(), agent_client_protocol::schema::v1::Error> {
    let cancelled = get_cancelled(sessions, &session_id.to_string());

    send_text(cx, session_id, &format!("Searching for: {query}..."))?;

    let results = step_search(cx, manager, session_id, "research", query, wiki_name, 5)?;

    if cancelled
        .as_ref()
        .map(|c| c.load(Ordering::Relaxed))
        .unwrap_or(false)
    {
        send_text(cx, session_id, "Cancelled.")?;
        clear_active_run(sessions, &session_id.to_string());
        return Ok(());
    }

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
            results[0].slug.as_str(),
            wiki_name,
            false,
        )?;
        step_report_results(cx, session_id, &results, wiki_name)?;
    }

    clear_active_run(sessions, &session_id.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use agent_client_protocol::schema::v1::SessionId;
    use agent_client_protocol::{Agent, Builder, Channel};

    use super::{step_report_results, step_search};

    fn make_engine() -> (crate::engine::WikiEngine, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("llm-wiki.toml");
        std::fs::write(&config_path, "").unwrap();
        let engine = crate::engine::WikiEngine::build(&config_path).unwrap();
        (engine, dir)
    }

    fn make_page_ref(slug: &str, uri: &str, score: f32) -> crate::search::PageRef {
        crate::search::PageRef {
            slug: crate::slug::NormalizedSlug::from_normalized(slug.to_string()),
            uri: uri.to_string(),
            title: slug.to_string(),
            score,
            confidence: 1.0,
            excerpt: None,
            summary: None,
        }
    }

    #[tokio::test]
    async fn step_report_results_empty_is_noop() {
        let session_id = SessionId::new("sess");
        let (chan_a, _chan_b) = Channel::duplex();
        Builder::<Agent>::new(Agent)
            .connect_with(chan_a, async |cx| {
                let result = step_report_results(&cx, &session_id, &[], "wiki");
                assert!(result.is_ok());
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn step_report_results_non_empty_succeeds() {
        let session_id = SessionId::new("sess");
        let results = vec![
            make_page_ref("concepts/a", "wiki/concepts/a", 1.0),
            make_page_ref("concepts/b", "wiki/concepts/b", 0.8),
        ];
        let (chan_a, _chan_b) = Channel::duplex();
        Builder::<Agent>::new(Agent)
            .connect_with(chan_a, async |cx| {
                let result = step_report_results(&cx, &session_id, &results, "wiki");
                assert!(result.is_ok());
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn step_search_engine_error_returns_empty_vec() {
        let (engine, _dir) = make_engine();
        let session_id = SessionId::new("sess");
        let (chan_a, _chan_b) = Channel::duplex();
        Builder::<Agent>::new(Agent)
            .connect_with(chan_a, async |cx| {
                let result =
                    step_search(&cx, &engine, &session_id, "research", "rust", "no-such-wiki", 5);
                assert!(result.is_ok());
                assert!(result.unwrap().is_empty());
                Ok(())
            })
            .await
            .unwrap();
    }
}
