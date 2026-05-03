use llm_wiki::graph::WikiGraphCache;

#[test]
fn wiki_graph_cache_no_snapshot_variant_exists() {
    let _ = std::mem::discriminant(&WikiGraphCache::NoSnapshot(
        petgraph_live::cache::GenerationCache::new(),
    ));
}
