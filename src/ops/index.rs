use anyhow::Result;

use crate::engine::{Engine, EngineManager};
use crate::indexing;

pub fn index_rebuild(manager: &EngineManager, wiki_name: &str) -> Result<indexing::IndexReport> {
    manager.rebuild_index(wiki_name)
}

pub fn index_status(engine: &Engine, wiki_name: &str) -> Result<indexing::IndexStatus> {
    let space = engine.space(wiki_name)?;
    indexing::index_status(wiki_name, &space.index_path, &space.repo_root)
}

