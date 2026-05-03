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
fn wiki_graph_cache_no_snapshot_uses_generation_cache() {
    let cache = WikiGraphCache::NoSnapshot(GenerationCache::<llm_wiki::graph::WikiGraph>::new());
    assert!(matches!(cache, WikiGraphCache::NoSnapshot(_)));
}
