use llm_wiki::acp::{dispatch_workflow, make_tool_id};

// ── Dispatch ──────────────────────────────────────────────────────────────────

#[test]
fn dispatch_prefix_research() {
    let (w, t) = dispatch_workflow("llm-wiki:research what is MoE?");
    assert_eq!(w, "research");
    assert_eq!(t, "what is MoE?");
}

#[test]
fn dispatch_prefix_ingest() {
    let (w, t) = dispatch_workflow("llm-wiki:ingest /path/to/file");
    assert_eq!(w, "ingest");
    assert_eq!(t, "/path/to/file");
}

#[test]
fn dispatch_prefix_no_text() {
    let (w, t) = dispatch_workflow("llm-wiki:research");
    assert_eq!(w, "research");
    assert_eq!(t, "");
}

#[test]
fn dispatch_no_prefix_falls_back_to_research() {
    let (w, t) = dispatch_workflow("what do we know about transformers?");
    assert_eq!(w, "research");
    assert_eq!(t, "what do we know about transformers?");
}

#[test]
fn dispatch_prefix_unknown_workflow() {
    let (w, t) = dispatch_workflow("llm-wiki:foobar some text");
    assert_eq!(w, "foobar");
    assert_eq!(t, "some text");
}

#[test]
fn dispatch_prefix_with_extra_spaces() {
    let (w, t) = dispatch_workflow("llm-wiki:  research   spaced query");
    assert_eq!(w, "research");
    assert_eq!(t, "spaced query");
}

#[test]
fn dispatch_lint_no_args() {
    let (wf, q) = dispatch_workflow("llm-wiki:lint");
    assert_eq!(wf, "lint");
    assert_eq!(q, "");
}

#[test]
fn dispatch_lint_with_rules() {
    let (wf, q) = dispatch_workflow("llm-wiki:lint orphan,stale");
    assert_eq!(wf, "lint");
    assert_eq!(q, "orphan,stale");
}

#[test]
fn dispatch_graph_no_root() {
    let (wf, q) = dispatch_workflow("llm-wiki:graph");
    assert_eq!(wf, "graph");
    assert_eq!(q, "");
}

#[test]
fn dispatch_graph_with_root() {
    let (wf, q) = dispatch_workflow("llm-wiki:graph concepts/transformer");
    assert_eq!(wf, "graph");
    assert_eq!(q, "concepts/transformer");
}

#[test]
fn dispatch_ingest_no_path() {
    let (wf, q) = dispatch_workflow("llm-wiki:ingest");
    assert_eq!(wf, "ingest");
    assert_eq!(q, "");
}

#[test]
fn dispatch_use() {
    let (wf, q) = dispatch_workflow("llm-wiki:use concepts/moe");
    assert_eq!(wf, "use");
    assert_eq!(q, "concepts/moe");
}

#[test]
fn dispatch_use_no_slug() {
    let (wf, q) = dispatch_workflow("llm-wiki:use");
    assert_eq!(wf, "use");
    assert_eq!(q, "");
}

#[test]
fn dispatch_help() {
    let (wf, q) = dispatch_workflow("llm-wiki:help");
    assert_eq!(wf, "help");
    assert_eq!(q, "");
}

// ── Tool ID ───────────────────────────────────────────────────────────────────

#[test]
fn make_tool_id_format() {
    let id = make_tool_id("research", "search");
    assert!(id.starts_with("research-search-"));
    let ts = &id["research-search-".len()..];
    assert!(ts.parse::<u64>().is_ok());
}

#[test]
fn make_tool_id_unique() {
    let id1 = make_tool_id("a", "b");
    std::thread::sleep(std::time::Duration::from_millis(2));
    let id2 = make_tool_id("a", "b");
    assert_ne!(id1, id2);
}
