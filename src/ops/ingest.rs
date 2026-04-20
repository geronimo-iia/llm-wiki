use std::path::Path;

use anyhow::Result;

use crate::engine::{Engine, EngineManager};
use crate::ingest;

pub fn ingest(
    engine: &Engine,
    manager: &EngineManager,
    path: &str,
    dry_run: bool,
    wiki_name: &str,
) -> Result<ingest::IngestReport> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let opts = ingest::IngestOptions {
        dry_run,
        auto_commit: resolved.ingest.auto_commit,
    };
    let report = ingest::ingest(
        Path::new(path),
        &opts,
        &space.wiki_root,
        &space.type_registry,
        &resolved.validation,
    )?;

    if !dry_run {
        if let Err(e) = manager.refresh_index(wiki_name) {
            tracing::warn!(error = %e, "incremental index update failed after ingest");
        }
    }

    Ok(report)
}

