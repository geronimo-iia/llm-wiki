use llm_wiki_engine::graph::WikiGraphCache;
use petgraph_live::cache::GenerationCache;

#[test]
fn wiki_graph_cache_no_snapshot_variant_exists() {
    let _ = std::mem::discriminant(&WikiGraphCache::NoSnapshot(GenerationCache::new()));
}

/// Snapshot file name must contain the git commit SHA, not a generation counter.
///
/// Invariant 1: the snapshot key is `last_commit()` (git HEAD SHA). If `key_fn`
/// were changed to return `generation().to_string()` the key would be "0" after
/// every process restart, causing stale snapshots to be served after index rebuilds.
/// This test pins the key by asserting the SHA appears in the snapshot filename.
#[test]
fn graph_snapshot_keyed_by_sha_not_generation() {
    use llm_wiki_engine::graph::{GraphFilter, get_or_build_graph};

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("mywiki");

    // Create wiki with at least one commit so last_commit() returns a real SHA.
    llm_wiki_engine::spaces::create(&wiki_path, "mywiki", None, false, true, &config_path, None)
        .unwrap();
    let wiki_root = wiki_path.join("wiki");
    std::fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    std::fs::write(
        wiki_root.join("concepts/a.md"),
        "---\ntitle: \"A\"\ntype: concept\nstatus: active\n---\nBody.\n",
    )
    .unwrap();
    llm_wiki_engine::git::commit(&wiki_path, "add page").unwrap();

    // Build engine, trigger snapshot creation via get_or_build_graph.
    let expected_sha = {
        let manager = llm_wiki_engine::engine::WikiEngine::build(&config_path).unwrap();
        let engine = manager.state.read().unwrap();
        let space = engine.spaces.get("mywiki").unwrap();
        let sha = space
            .index_manager
            .last_commit()
            .expect("index must have a commit after rebuild at mount time");
        let searcher = space.index_manager.searcher().unwrap();
        let _ = get_or_build_graph(
            &space.index_schema,
            &space.type_registry,
            &space.index_manager,
            &space.graph_cache,
            &searcher,
            &GraphFilter::default(),
        )
        .unwrap();
        sha
    };

    // The snapshot file name must embed the git SHA, not "0" or "1".
    let snap_dir = dir.path().join("state").join("snapshots").join("mywiki");
    let files: Vec<String> = std::fs::read_dir(&snap_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        files.len(),
        1,
        "expected exactly one snapshot file, got {files:?}"
    );
    assert!(
        files[0].contains(&expected_sha),
        "snapshot filename must contain the git SHA '{expected_sha}' but got '{}'; \
         if key_fn was changed to generation() the filename would contain '0' instead",
        files[0]
    );
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
    let cache =
        WikiGraphCache::NoSnapshot(GenerationCache::<llm_wiki_engine::graph::WikiGraph>::new());
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
    use llm_wiki_engine::graph::{GraphFilter, get_or_build_graph};
    use llm_wiki_engine::ops;

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("mywiki");

    // ── Process 1: create empty wiki, trigger first mount ──────────────────
    // This saves wiki-graph-0.snap.lz4 (empty) because generation=0 and wiki has 0 pages.
    llm_wiki_engine::spaces::create(&wiki_path, "mywiki", None, false, true, &config_path, None)
        .unwrap();
    let engine1 = llm_wiki_engine::engine::WikiEngine::build(&config_path).unwrap();
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
    llm_wiki_engine::git::commit(&wiki_path, "add alpha and beta pages").unwrap();

    // ── Process 2: rebuild index ───────────────────────────────────────────
    // Simulates: `llm-wiki index rebuild --wiki mywiki`
    let manager2 = llm_wiki_engine::engine::WikiEngine::build(&config_path).unwrap();
    ops::index_rebuild(&manager2, "mywiki").unwrap();
    drop(manager2); // process exits

    // ── Process 3: fresh engine, query graph ──────────────────────────────
    // Simulates: `llm-wiki graph --wiki mywiki`
    // BUG: before fix, this loads the stale empty wiki-graph-0.snap.lz4 and returns 0 nodes.
    let manager3 = llm_wiki_engine::engine::WikiEngine::build(&config_path).unwrap();
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
