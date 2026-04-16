use std::fs;
use std::path::Path;
use std::sync::Arc;

use agent_client_protocol as acp;
use tokio::sync::{mpsc, oneshot};

use llm_wiki::acp::WikiAgent;
use llm_wiki::config::{GlobalConfig, WikiEntry};
use llm_wiki::git;

fn setup_wiki(dir: &Path) -> GlobalConfig {
    let wiki_root = dir.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::create_dir_all(dir.join("inbox")).unwrap();
    fs::create_dir_all(dir.join("raw")).unwrap();
    git::init_repo(dir).unwrap();
    fs::write(dir.join("README.md"), "# test\n").unwrap();

    fs::write(
        wiki_root.join("concepts/moe.md"),
        "---\ntitle: \"Mixture of Experts\"\nsummary: \"MoE scaling\"\nstatus: active\n\
         last_updated: \"2025-01-01\"\ntype: concept\ntags:\n  - scaling\n---\n\nMoE scales.\n",
    )
    .unwrap();

    git::commit(dir, "init").unwrap();

    GlobalConfig {
        global: llm_wiki::config::GlobalSection {
            default_wiki: "test".to_string(),
        },
        wikis: vec![WikiEntry {
            name: "test".to_string(),
            path: dir.to_string_lossy().to_string(),
            description: None,
            remote: None,
        }],
        ..Default::default()
    }
}

fn make_agent(
    global: GlobalConfig,
) -> (
    WikiAgent,
    mpsc::UnboundedReceiver<(acp::SessionNotification, oneshot::Sender<()>)>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let agent = WikiAgent::new(Arc::new(global), tx);
    (agent, rx)
}

/// Extract text messages from a list of SessionUpdate.
fn extract_messages(updates: &[acp::SessionUpdate]) -> Vec<String> {
    updates
        .iter()
        .filter_map(|u| match u {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(t) = &chunk.content {
                    Some(t.text.clone())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect()
}

/// Extract ToolCall events from a list of SessionUpdate.
#[allow(dead_code)]
fn extract_tool_calls(updates: &[acp::SessionUpdate]) -> Vec<&acp::ToolCall> {
    updates
        .iter()
        .filter_map(|u| match u {
            acp::SessionUpdate::ToolCall(tc) => Some(tc),
            _ => None,
        })
        .collect()
}

/// Extract ToolCallUpdate events from a list of SessionUpdate.
#[allow(dead_code)]
fn extract_tool_updates(updates: &[acp::SessionUpdate]) -> Vec<&acp::ToolCallUpdate> {
    updates
        .iter()
        .filter_map(|u| match u {
            acp::SessionUpdate::ToolCallUpdate(tcu) => Some(tcu),
            _ => None,
        })
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn initialize_injects_instructions() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, _rx) = make_agent(global);

    let req = acp::InitializeRequest::new(acp::ProtocolVersion::LATEST);
    let resp = acp::Agent::initialize(&agent, req).await.unwrap();

    assert!(resp.agent_info.is_some());
    let info = resp.agent_info.unwrap();
    assert_eq!(info.name, "llm-wiki");

    let meta = resp.meta.unwrap();
    let system = meta.get("system").unwrap().as_str().unwrap();
    assert!(system.contains("Session orientation"));
    assert!(system.contains("Linking policy"));
}

#[tokio::test(flavor = "current_thread")]
async fn new_session_and_list_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, _rx) = make_agent(global);

    let req = acp::NewSessionRequest::new(".");
    let resp = acp::Agent::new_session(&agent, req).await.unwrap();
    let sid = resp.session_id.to_string();
    assert!(sid.starts_with("session-"));

    let list = acp::Agent::list_sessions(&agent, acp::ListSessionsRequest::new())
        .await
        .unwrap();
    assert_eq!(list.sessions.len(), 1);
    assert_eq!(list.sessions[0].session_id.to_string(), sid);
}

#[tokio::test(flavor = "current_thread")]
async fn load_session_existing_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, _rx) = make_agent(global);

    let new_resp = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let load_req = acp::LoadSessionRequest::new(new_resp.session_id.clone(), ".");
    let result = acp::Agent::load_session(&agent, load_req).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn load_session_missing_fails() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, _rx) = make_agent(global);

    let load_req = acp::LoadSessionRequest::new("nonexistent", ".");
    let result = acp::Agent::load_session(&agent, load_req).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn prompt_research_workflow_streams_answer() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());

    // Build index so search works
    let wiki_root = dir.path().join("wiki");
    let index_path = dir.path().join("index-store");
    llm_wiki::search::rebuild_index(&wiki_root, &index_path, "test", dir.path()).unwrap();

    let (agent, rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let drain = tokio::task::spawn_local(drain_updates(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "what do you know about MoE scaling?",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let updates = drain.await.unwrap();

            // Should have multiple streaming events
            assert!(
                updates.len() >= 2,
                "research workflow should stream multiple events, got {}",
                updates.len()
            );

            // First should be a message ("Searching for...")
            assert!(
                matches!(&updates[0], acp::SessionUpdate::AgentMessageChunk(_)),
                "first event should be a progress message"
            );

            // Should contain at least one ToolCall
            assert!(
                updates.iter().any(|u| matches!(u, acp::SessionUpdate::ToolCall(_))),
                "should contain at least one ToolCall"
            );

            // Should contain at least one ToolCallUpdate
            assert!(
                updates.iter().any(|u| matches!(u, acp::SessionUpdate::ToolCallUpdate(_))),
                "should contain at least one ToolCallUpdate"
            );

            // Last should be a message (summary or no results)
            assert!(
                matches!(updates.last().unwrap(), acp::SessionUpdate::AgentMessageChunk(_)),
                "last event should be a message"
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn prompt_ingest_workflow_dispatches_on_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let drain = tokio::task::spawn_local(drain_updates(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "ingest the semantic-commit skill",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let updates = drain.await.unwrap();
            let messages = extract_messages(&updates);
            assert!(
                !messages.is_empty(),
                "ingest workflow should stream a message"
            );
            assert!(
                messages[0].contains("Ingest workflow triggered"),
                "should dispatch to ingest: {}",
                messages[0]
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn prompt_lint_workflow_streams_tool_calls() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let drain = tokio::task::spawn_local(drain_updates(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "run lint on research wiki",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let updates = drain.await.unwrap();

            // Should have: message + ToolCall + ToolCallUpdate + message = 4
            assert!(
                updates.len() >= 3,
                "lint workflow should stream multiple events, got {}",
                updates.len()
            );

            // First is a progress message
            assert!(
                matches!(&updates[0], acp::SessionUpdate::AgentMessageChunk(_)),
                "first event should be a progress message"
            );

            // Should contain a ToolCall
            assert!(
                updates.iter().any(|u| matches!(u, acp::SessionUpdate::ToolCall(_))),
                "should contain a ToolCall"
            );

            // Should contain a ToolCallUpdate
            assert!(
                updates.iter().any(|u| matches!(u, acp::SessionUpdate::ToolCallUpdate(_))),
                "should contain a ToolCallUpdate"
            );

            // Last is the report message
            if let acp::SessionUpdate::AgentMessageChunk(chunk) = updates.last().unwrap() {
                if let acp::ContentBlock::Text(t) = &chunk.content {
                    assert!(
                        t.text.contains("Lint report") || t.text.contains("Lint failed"),
                        "last message should be lint report: {}",
                        t.text
                    );
                }
            }
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn prompt_crystallize_workflow_dispatches_on_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let drain = tokio::task::spawn_local(drain_updates(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "crystallize session insights",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let updates = drain.await.unwrap();
            let messages = extract_messages(&updates);
            assert!(
                !messages.is_empty(),
                "crystallize workflow should stream a message"
            );
            assert!(
                messages[0].contains("Crystallize workflow triggered"),
                "should dispatch to crystallize: {}",
                messages[0]
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancel_clears_active_run() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, _rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let cancel = acp::CancelNotification::new(session.session_id.clone());
    let result = acp::Agent::cancel(&agent, cancel).await;
    assert!(result.is_ok());
}

/// Collect all SessionUpdate variants from the notification channel.
async fn drain_updates(
    mut rx: mpsc::UnboundedReceiver<(acp::SessionNotification, oneshot::Sender<()>)>,
) -> Vec<acp::SessionUpdate> {
    let mut updates = Vec::new();
    while let Some((notif, tx)) = rx.recv().await {
        updates.push(notif.update);
        tx.send(()).ok();
    }
    updates
}

#[tokio::test(flavor = "current_thread")]
async fn send_tool_call_and_result_appear_in_channel() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let drain = tokio::task::spawn_local(drain_updates(rx));

            let tool_id = WikiAgent::make_tool_id("test", "search");

            agent
                .send_tool_call(
                    &session.session_id,
                    &tool_id,
                    "wiki_search: MoE",
                    acp::ToolKind::Search,
                )
                .await
                .unwrap();

            agent
                .send_tool_result(
                    &session.session_id,
                    &tool_id,
                    acp::ToolCallStatus::Completed,
                    "3 results",
                )
                .await
                .unwrap();

            agent
                .send_message(&session.session_id, "Done")
                .await
                .unwrap();

            drop(agent);
            let updates = drain.await.unwrap();

            assert_eq!(updates.len(), 3);

            assert!(
                matches!(&updates[0], acp::SessionUpdate::ToolCall(_)),
                "first update should be ToolCall"
            );
            assert!(
                matches!(&updates[1], acp::SessionUpdate::ToolCallUpdate(_)),
                "second update should be ToolCallUpdate"
            );
            assert!(
                matches!(&updates[2], acp::SessionUpdate::AgentMessageChunk(_)),
                "third update should be AgentMessageChunk"
            );

            // Verify ToolCall fields
            if let acp::SessionUpdate::ToolCall(tc) = &updates[0] {
                assert!(tc.tool_call_id.to_string().starts_with("test-search-"));
                assert_eq!(tc.title, "wiki_search: MoE");
            }

            // Verify ToolCallUpdate fields
            if let acp::SessionUpdate::ToolCallUpdate(tcu) = &updates[1] {
                assert_eq!(
                    tcu.fields.status,
                    Some(acp::ToolCallStatus::Completed)
                );
            }
        })
        .await;
}

#[test]
fn make_tool_id_has_correct_format() {
    let id = WikiAgent::make_tool_id("research", "search");
    assert!(id.starts_with("research-search-"));
    // Timestamp part should be numeric
    let parts: Vec<&str> = id.splitn(3, '-').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[2].parse::<u64>().is_ok());
}
