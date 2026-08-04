use anyhow::Result;

use crate::engine::{EngineState, WikiEngine};
use crate::index_manager;

/// Tear down and rebuild the tantivy index for the named wiki.
pub fn index_rebuild(manager: &WikiEngine, wiki_name: &str) -> Result<index_manager::IndexReport> {
    let report = manager.rebuild_index(wiki_name)?;

    // Non-fatal: refresh the graph snapshot after index rebuild.
    let engine = manager
        .state
        .read()
        .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
    if let Ok(space) = engine.space(wiki_name) {
        let current_gen = space.index_manager.generation();
        if let Ok(searcher) = space.index_manager.searcher() {
            let _ = space.graph_cache.rebuild(current_gen, || {
                crate::graph::build_graph(
                    &searcher,
                    &space.index_schema,
                    &crate::graph::GraphFilter::default(),
                    &space.type_registry,
                )
            });
        }
    }

    Ok(report)
}

/// Return the health and staleness status of the named wiki's index.
pub fn index_status(engine: &EngineState, wiki_name: &str) -> Result<index_manager::IndexStatus> {
    let space = engine.space(wiki_name)?;
    space.index_manager.status(&space.repo_root)
}
