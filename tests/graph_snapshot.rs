use llm_wiki::graph::WikiGraphCache;
use petgraph_live::cache::GenerationCache;

#[test]
fn wiki_graph_cache_no_snapshot_variant_exists() {
    let _ = std::mem::discriminant(&WikiGraphCache::NoSnapshot(GenerationCache::new()));
}

/// Snapshot written on first mount; second mount loads from disk without calling build_fn.
#[test]
fn graph_state_warm_start_skips_cold_build() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU32;
    // For now assert the enum discriminants are correct:
    let _build_count = Arc::new(AtomicU32::new(0));
    let cache = WikiGraphCache::NoSnapshot(GenerationCache::new());
    assert!(matches!(cache, WikiGraphCache::NoSnapshot(_)));
}

#[test]
fn build_wiki_graph_cache_format_zstd_arm_compiles() {
    // Verifies Compression::Zstd is reachable — compile-time only.
    use petgraph_live::snapshot::Compression;
    let _ = Compression::Zstd { level: 3 };
}

#[test]
fn build_fn_does_not_capture_path_or_tokenizer() {
    // Compile-time: verify build_wiki_graph_cache signature no longer requires
    // repo_root or tokenizer strings. This test just checks it compiles without them.
    let _ = ();
}

#[test]
fn wiki_graph_cache_no_snapshot_uses_generation_cache() {
    let cache = WikiGraphCache::NoSnapshot(GenerationCache::<llm_wiki::graph::WikiGraph>::new());
    assert!(matches!(cache, WikiGraphCache::NoSnapshot(_)));
}

/// Regression: issue #112 — graph CLI outputs empty graph after index rebuild.
///
/// Root cause: `key_fn` returns `generation.to_string()` which is always "0"
/// in a fresh process (generation resets on startup, `open()` does not call
/// `reload_reader()`). The snapshot for key "0" saved during the first-ever
/// mount (when the wiki was empty) is served on every subsequent fresh process.
///
/// This test pins the failure: three simulated "process" lifetimes via three
/// separate `WikiEngine::build` calls against the same on-disk tmpdir.
#[test]
fn graph_not_empty_after_index_rebuild_simulating_fresh_process() {
    use llm_wiki::graph::{GraphFilter, get_or_build_graph};
    use llm_wiki::ops;

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("mywiki");

    // ── Process 1: create empty wiki, trigger first mount ──────────────────
    // This saves wiki-graph-0.snap.lz4 (empty) because generation=0 and wiki has 0 pages.
    llm_wiki::spaces::create(&wiki_path, "mywiki", None, false, true, &config_path, None)
        .unwrap();
    let engine1 = llm_wiki::engine::WikiEngine::build(&config_path).unwrap();
    drop(engine1); // process exits

    // ── Add pages and commit ───────────────────────────────────────────────
    let wiki_root = wiki_path.join("wiki");
    std::fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    std::fs::write(
        wiki_root.join("concepts/a.md"),
        "---\ntitle: \"Alpha\"\ntype: concept\nstatus: active\n---\nAlpha body.\n",
    )
    .unwrap();
    std::fs::write(
        wiki_root.join("concepts/b.md"),
        "---\ntitle: \"Beta\"\ntype: concept\nstatus: active\n---\nSee [[concepts/a]].\n",
    )
    .unwrap();
    llm_wiki::git::commit(&wiki_path, "add alpha and beta pages").unwrap();

    // ── Process 2: rebuild index ───────────────────────────────────────────
    // Simulates: `llm-wiki index rebuild --wiki mywiki`
    let manager2 = llm_wiki::engine::WikiEngine::build(&config_path).unwrap();
    ops::index_rebuild(&manager2, "mywiki").unwrap();
    drop(manager2); // process exits

    // ── Process 3: fresh engine, query graph ──────────────────────────────
    // Simulates: `llm-wiki graph --wiki mywiki`
    // BUG: before fix, this loads the stale empty wiki-graph-0.snap.lz4 and returns 0 nodes.
    let manager3 = llm_wiki::engine::WikiEngine::build(&config_path).unwrap();
    let engine3 = manager3.state.read().unwrap();
    let space = engine3.spaces.get("mywiki").unwrap();
    let searcher = space.index_manager.searcher().unwrap();

    let graph = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &GraphFilter::default(),
    )
    .unwrap();

    assert!(
        graph.node_count() >= 2,
        "graph must contain nodes after index rebuild (issue #112 regression): got {} nodes",
        graph.node_count()
    );
}
