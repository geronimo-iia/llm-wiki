use std::fs;
use std::path::Path;
use std::sync::Arc;

use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::graph::{GraphFilter, get_cached_community_map, get_or_build_graph};

fn setup_wiki(dir: &Path, name: &str) -> std::path::PathBuf {
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join(name);

    llm_wiki::spaces::create(&wiki_path, name, None, false, true, &config_path).unwrap();

    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/moe.md"),
        "---\ntitle: \"MoE\"\ntype: concept\nstatus: active\ntags: [ml]\n---\n\nMixture of Experts.\n",
    )
    .unwrap();
    fs::write(
        wiki_root.join("concepts/transformer.md"),
        "---\ntitle: \"Transformer\"\ntype: concept\nstatus: active\n---\n\nAttention is all you need. See [[concepts/moe]].\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add pages").unwrap();

    config_path
}

#[test]
fn graph_cache_hit_returns_same_arc() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.spaces.get("test").unwrap();

    let searcher = space.index_manager.searcher().unwrap();
    let filter = GraphFilter::default();

    let g1 = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &filter,
    )
    .unwrap();

    let g2 = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &filter,
    )
    .unwrap();

    assert!(
        Arc::ptr_eq(&g1, &g2),
        "second call should return cached Arc"
    );
}

#[test]
fn graph_cache_miss_on_filtered_request() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.spaces.get("test").unwrap();

    let searcher = space.index_manager.searcher().unwrap();

    // Build and cache the full graph
    let full = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &GraphFilter::default(),
    )
    .unwrap();

    // Filtered request should bypass cache
    let filtered = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &GraphFilter {
            types: vec!["concept".to_string()],
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        !Arc::ptr_eq(&full, &filtered),
        "filtered request must not return cached full graph"
    );
}

#[test]
fn graph_cache_hit_is_faster_than_miss() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.spaces.get("test").unwrap();

    let searcher = space.index_manager.searcher().unwrap();
    let filter = GraphFilter::default();

    // Cold call — cache miss, builds graph
    let t0 = std::time::Instant::now();
    let _ = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &filter,
    )
    .unwrap();
    let cold_ns = t0.elapsed().as_nanos();

    // Warm call — cache hit, returns Arc clone
    let t1 = std::time::Instant::now();
    let _ = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &filter,
    )
    .unwrap();
    let warm_ns = t1.elapsed().as_nanos();

    // Cache hit must be strictly faster than cache miss
    assert!(
        warm_ns < cold_ns,
        "cache hit ({warm_ns}ns) not faster than miss ({cold_ns}ns)"
    );
}

#[test]
fn get_cached_community_map_returns_none_for_small_graph() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.spaces.get("test").unwrap();

    let searcher = space.index_manager.searcher().unwrap();

    // With only 2 nodes, community detection should return None
    let map = get_cached_community_map(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        30,
    )
    .unwrap();

    assert!(map.is_none(), "graph too small for community detection");
}
