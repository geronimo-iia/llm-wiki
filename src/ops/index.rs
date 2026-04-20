use anyhow::Result;

use crate::engine::{EngineState, WikiEngine};
use crate::index_manager;

pub fn index_rebuild(manager: &WikiEngine, wiki_name: &str) -> Result<index_manager::IndexReport> {
    manager.rebuild_index(wiki_name)
}

pub fn index_status(engine: &EngineState, wiki_name: &str) -> Result<index_manager::IndexStatus> {
    let space = engine.space(wiki_name)?;
    space.index_manager.status(&space.repo_root)
}
