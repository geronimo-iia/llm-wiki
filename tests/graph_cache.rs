use std::fs;
use std::path::Path;
use std::sync::Arc;

use petgraph::visit::EdgeRef;

use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::graph::{
    GraphFilter, compute_communities, get_cached_community_map, get_cached_community_stats,
    get_or_build_graph, merge_cached_graphs, node_community_map,
};

fn setup_wiki(dir: &Path, name: &str) -> std::path::PathBuf {
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join(name);

    llm_wiki::spaces::create(&wiki_path, name, None, false, true, &config_path, None).unwrap();

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
        &space.community_cache,
        &searcher,
        30,
    )
    .unwrap();

    assert!(map.is_none(), "graph too small for community detection");
}

#[test]
fn get_cached_community_stats_returns_none_for_small_graph() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.spaces.get("test").unwrap();
    let searcher = space.index_manager.searcher().unwrap();

    // Test wiki has only 2 nodes — below threshold of 30
    let stats = get_cached_community_stats(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &space.community_cache,
        &searcher,
        30,
    )
    .unwrap();
    assert!(stats.is_none());
}

/// Helper: create a wiki space at `dir/name` with given pages, sharing `config_path`.
fn setup_space(dir: &Path, name: &str, config_path: &Path, pages: &[(&str, &str)]) {
    let wiki_path = dir.join(name);
    llm_wiki::spaces::create(&wiki_path, name, None, false, true, config_path, None).unwrap();
    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    for (filename, content) in pages {
        let file_path = wiki_root.join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, content).unwrap();
    }
    git::commit(&wiki_path, "add pages").unwrap();
}

#[test]
fn cross_wiki_merge_cached_graphs_matches_build_graph_cross_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");

    // Wiki A: has a page that links cross-wiki to wiki B
    setup_space(
        dir.path(),
        "alpha",
        &config_path,
        &[
            (
                "concepts/foo.md",
                "---\ntitle: \"Foo\"\ntype: concept\nstatus: active\n---\n\nSee [[wiki://beta/concepts/bar]].\n",
            ),
            (
                "concepts/baz.md",
                "---\ntitle: \"Baz\"\ntype: concept\nstatus: active\n---\n\nLocal link to [[concepts/foo]].\n",
            ),
        ],
    );

    // Wiki B: has a page targeted by wiki A
    setup_space(
        dir.path(),
        "beta",
        &config_path,
        &[(
            "concepts/bar.md",
            "---\ntitle: \"Bar\"\ntype: concept\nstatus: active\n---\n\nBar content.\n",
        )],
    );

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let filter = GraphFilter::default();

    // Build via merge_cached_graphs (the new path)
    let mut per_space: Vec<(&str, Arc<llm_wiki::graph::WikiGraph>)> = Vec::new();
    for (name, sp) in engine.spaces.iter() {
        let searcher = sp.index_manager.searcher().unwrap();
        let g = get_or_build_graph(
            &sp.index_schema,
            &sp.type_registry,
            &sp.index_manager,
            &sp.graph_cache,
            &searcher,
            &filter,
        )
        .unwrap();
        per_space.push((name.as_str(), g));
    }
    // Sort for determinism
    per_space.sort_by_key(|(name, _)| name.to_string());
    let merged = merge_cached_graphs(&per_space, &filter).unwrap();

    // Verify: 3 local nodes (alpha/foo, alpha/baz, beta/bar)
    let local_nodes: Vec<_> = merged
        .node_indices()
        .filter(|&idx| !merged[idx].external)
        .collect();
    assert_eq!(
        local_nodes.len(),
        3,
        "expected 3 local nodes in merged graph"
    );

    // Verify: cross-wiki edge alpha/foo -> beta/bar is resolved (not external)
    let foo_idx = merged
        .node_indices()
        .find(|&idx| merged[idx].slug == "alpha/concepts/foo")
        .expect("alpha/concepts/foo node must exist");
    let bar_idx = merged
        .node_indices()
        .find(|&idx| merged[idx].slug == "beta/concepts/bar")
        .expect("beta/concepts/bar node must exist");

    let has_cross_edge = merged.edges(foo_idx).any(|e| e.target() == bar_idx);
    assert!(
        has_cross_edge,
        "cross-wiki edge from alpha/concepts/foo to beta/concepts/bar must be resolved"
    );

    // Verify: no external placeholders (both wikis mounted)
    let external_count = merged
        .node_indices()
        .filter(|&idx| merged[idx].external)
        .count();
    assert_eq!(
        external_count, 0,
        "no external nodes when both wikis are mounted"
    );
}

#[test]
fn cross_wiki_merge_keeps_external_when_target_wiki_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");

    // Only wiki A — references wiki "gamma" which is not mounted
    setup_space(
        dir.path(),
        "alpha",
        &config_path,
        &[(
            "concepts/foo.md",
            "---\ntitle: \"Foo\"\ntype: concept\nstatus: active\n---\n\nSee [[wiki://gamma/concepts/missing]].\n",
        )],
    );

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let filter = GraphFilter::default();

    let sp = engine.spaces.get("alpha").unwrap();
    let searcher = sp.index_manager.searcher().unwrap();
    let g = get_or_build_graph(
        &sp.index_schema,
        &sp.type_registry,
        &sp.index_manager,
        &sp.graph_cache,
        &searcher,
        &filter,
    )
    .unwrap();

    let per_space = vec![("alpha", g)];
    let merged = merge_cached_graphs(&per_space, &filter).unwrap();

    // The cross-wiki target should remain as an external placeholder
    let external_nodes: Vec<_> = merged
        .node_indices()
        .filter(|&idx| merged[idx].external)
        .collect();
    assert_eq!(
        external_nodes.len(),
        1,
        "unmounted target wiki should produce external placeholder"
    );
}

#[test]
fn community_stats_and_map_are_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.spaces.get("test").unwrap();
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

    // threshold=1: both wiki's 2 nodes should be >= 1
    let stats = compute_communities(&graph, 1);
    let map = node_community_map(&graph, 1);

    assert!(stats.is_some(), "expected Some(CommunityStats)");
    assert!(map.is_some(), "expected Some(community_map)");

    let stats = stats.unwrap();
    let map = map.unwrap();

    // every community id in map must be < stats.count
    for &c in map.values() {
        assert!(
            c < stats.count,
            "community id {c} out of range (count={})",
            stats.count
        );
    }
}

#[test]
fn graph_cache_invalidated_after_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();

    let arc1 = {
        let engine = manager.state.read().unwrap();
        let space = engine.spaces.get("test").unwrap();
        let searcher = space.index_manager.searcher().unwrap();
        get_or_build_graph(
            &space.index_schema,
            &space.type_registry,
            &space.index_manager,
            &space.graph_cache,
            &searcher,
            &GraphFilter::default(),
        )
        .unwrap()
    }; // drop engine read lock

    // Trigger rebuild — bumps generation
    {
        let engine = manager.state.read().unwrap();
        let space = engine.spaces.get("test").unwrap();
        space
            .index_manager
            .rebuild(
                &space.wiki_root,
                &space.repo_root,
                &space.index_schema,
                &space.type_registry,
            )
            .unwrap();
    }

    let arc2 = {
        let engine = manager.state.read().unwrap();
        let space = engine.spaces.get("test").unwrap();
        let searcher = space.index_manager.searcher().unwrap();
        get_or_build_graph(
            &space.index_schema,
            &space.type_registry,
            &space.index_manager,
            &space.graph_cache,
            &searcher,
            &GraphFilter::default(),
        )
        .unwrap()
    };

    assert!(
        !Arc::ptr_eq(&arc1, &arc2),
        "cache must be invalidated after rebuild"
    );
}

/// Cache always runs Louvain at threshold=0, so accessors only check local node count vs min_nodes.
/// Regression: previously the cache was built at threshold=30, causing None for graphs with 5–29 nodes.
#[test]
fn get_cached_community_stats_returns_some_for_graph_above_min_nodes_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");

    // Build a wiki with 6 nodes (> 5, < 30)
    setup_space(
        dir.path(),
        "test",
        &config_path,
        &[
            (
                "concepts/a.md",
                "---\ntitle: \"A\"\ntype: concept\nstatus: active\n---\nSee [[concepts/b]].\n",
            ),
            (
                "concepts/b.md",
                "---\ntitle: \"B\"\ntype: concept\nstatus: active\n---\nSee [[concepts/c]].\n",
            ),
            (
                "concepts/c.md",
                "---\ntitle: \"C\"\ntype: concept\nstatus: active\n---\nSee [[concepts/d]].\n",
            ),
            (
                "concepts/d.md",
                "---\ntitle: \"D\"\ntype: concept\nstatus: active\n---\nSee [[concepts/e]].\n",
            ),
            (
                "concepts/e.md",
                "---\ntitle: \"E\"\ntype: concept\nstatus: active\n---\nSee [[concepts/f]].\n",
            ),
            (
                "concepts/f.md",
                "---\ntitle: \"F\"\ntype: concept\nstatus: active\n---\nLeaf node.\n",
            ),
        ],
    );

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.spaces.get("test").unwrap();
    let searcher = space.index_manager.searcher().unwrap();

    // threshold=5: 6 nodes >= 5, should return Some
    let stats = get_cached_community_stats(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &space.community_cache,
        &searcher,
        5,
    )
    .unwrap();
    assert!(
        stats.is_some(),
        "expected Some(CommunityStats) for 6-node graph with min_nodes=5, got None"
    );

    // Second call — cache hit path must also return Some
    let stats2 = get_cached_community_stats(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &space.community_cache,
        &searcher,
        5,
    )
    .unwrap();
    assert!(stats2.is_some(), "cache-hit path must also return Some");
}
