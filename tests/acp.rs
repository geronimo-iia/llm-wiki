use std::fs;
use std::path::Path;
use std::sync::Arc;

use llm_wiki::acp::WikiAgent;
use llm_wiki::engine::WikiEngine;
use llm_wiki::git;

use tokio::sync::mpsc;

fn setup_wiki(dir: &Path, name: &str) -> std::path::PathBuf {
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join(name);

    llm_wiki::spaces::create(&wiki_path, name, None, false, true, &config_path).unwrap();

    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/moe.md"),
        "---\ntitle: \"MoE\"\ntype: concept\nstatus: active\n---\n\nMixture of Experts.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add page").unwrap();

    config_path
}

fn make_agent(config_path: &Path) -> WikiAgent {
    let manager = Arc::new(WikiEngine::build(config_path).unwrap());
    let (tx, _rx) = mpsc::unbounded_channel();
    WikiAgent::new(manager, tx)
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

#[test]
fn dispatch_prefix_research() {
    let (w, t) = WikiAgent::dispatch_workflow("llm-wiki:research what is MoE?");
    assert_eq!(w, "research");
    assert_eq!(t, "what is MoE?");
}

#[test]
fn dispatch_prefix_ingest() {
    let (w, t) = WikiAgent::dispatch_workflow("llm-wiki:ingest /path/to/file");
    assert_eq!(w, "ingest");
    assert_eq!(t, "/path/to/file");
}

#[test]
fn dispatch_prefix_no_text() {
    let (w, t) = WikiAgent::dispatch_workflow("llm-wiki:research");
    assert_eq!(w, "research");
    assert_eq!(t, "");
}

#[test]
fn dispatch_no_prefix_falls_back_to_research() {
    let (w, t) = WikiAgent::dispatch_workflow("what do we know about transformers?");
    assert_eq!(w, "research");
    assert_eq!(t, "what do we know about transformers?");
}

#[test]
fn dispatch_prefix_unknown_workflow() {
    let (w, t) = WikiAgent::dispatch_workflow("llm-wiki:foobar some text");
    assert_eq!(w, "foobar");
    assert_eq!(t, "some text");
}

#[test]
fn dispatch_prefix_with_extra_spaces() {
    let (w, t) = WikiAgent::dispatch_workflow("llm-wiki:  research   spaced query");
    assert_eq!(w, "research");
    assert_eq!(t, "spaced query");
}

// ── Tool ID ───────────────────────────────────────────────────────────────────

#[test]
fn make_tool_id_format() {
    let id = WikiAgent::make_tool_id("research", "search");
    assert!(id.starts_with("research-search-"));
    // Timestamp portion should be numeric
    let ts = &id["research-search-".len()..];
    assert!(ts.parse::<u64>().is_ok());
}

#[test]
fn make_tool_id_unique() {
    let id1 = WikiAgent::make_tool_id("a", "b");
    std::thread::sleep(std::time::Duration::from_millis(2));
    let id2 = WikiAgent::make_tool_id("a", "b");
    assert_ne!(id1, id2);
}

// ── Session management ────────────────────────────────────────────────────────

#[test]
fn agent_creates_with_engine() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let agent = make_agent(&config_path);

    // Verify engine is accessible
    let engine = agent.manager.state.read().unwrap();
    assert!(engine.spaces.contains_key("test"));
}

#[test]
fn resolve_wiki_name_uses_default() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let agent = make_agent(&config_path);

    let name = agent.resolve_wiki_name(None);
    assert_eq!(name, "test");
}

#[test]
fn resolve_wiki_name_uses_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    // Create a second wiki
    let wiki2 = dir.path().join("other");
    llm_wiki::spaces::create(&wiki2, "other", None, false, false, &config_path).unwrap();

    let agent = make_agent(&config_path);
    let name = agent.resolve_wiki_name(Some("other"));
    assert_eq!(name, "other");
}
