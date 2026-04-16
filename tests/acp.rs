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

/// Collect all streamed text messages from the notification channel.
async fn drain_messages(
    mut rx: mpsc::UnboundedReceiver<(acp::SessionNotification, oneshot::Sender<()>)>,
) -> Vec<String> {
    let mut messages = Vec::new();
    while let Some((notif, tx)) = rx.recv().await {
        if let acp::SessionUpdate::AgentMessageChunk(chunk) = &notif.update {
            if let acp::ContentBlock::Text(t) = &chunk.content {
                messages.push(t.text.clone());
            }
        }
        tx.send(()).ok();
    }
    messages
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
    let (agent, rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let drain = tokio::task::spawn_local(drain_messages(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "what do you know about MoE scaling?",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let messages = drain.await.unwrap();
            assert!(
                !messages.is_empty(),
                "research workflow should stream a message"
            );
            let msg = &messages[0];
            assert!(
                msg.contains("results")
                    || msg.contains("Search failed")
                    || msg.contains("No results"),
                "research response should mention results: {msg}"
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
            let drain = tokio::task::spawn_local(drain_messages(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "ingest the semantic-commit skill",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let messages = drain.await.unwrap();
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
async fn prompt_lint_workflow_dispatches_on_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let global = setup_wiki(dir.path());
    let (agent, rx) = make_agent(global);

    let session = acp::Agent::new_session(&agent, acp::NewSessionRequest::new("."))
        .await
        .unwrap();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let drain = tokio::task::spawn_local(drain_messages(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "run lint on research wiki",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let messages = drain.await.unwrap();
            assert!(
                !messages.is_empty(),
                "lint workflow should stream a message"
            );
            assert!(
                messages[0].contains("Lint report") || messages[0].contains("Lint failed"),
                "should dispatch to lint: {}",
                messages[0]
            );
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
            let drain = tokio::task::spawn_local(drain_messages(rx));

            let prompt = vec![acp::ContentBlock::Text(acp::TextContent::new(
                "crystallize session insights",
            ))];
            let req = acp::PromptRequest::new(session.session_id.clone(), prompt);
            let resp = acp::Agent::prompt(&agent, req).await.unwrap();
            assert_eq!(resp.stop_reason, acp::StopReason::EndTurn);

            drop(agent);
            let messages = drain.await.unwrap();
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
