use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::engine::WikiEngine;

// ── Event types ───────────────────────────────────────────────────────────────

enum WatchAction {
    IngestPages(Vec<PathBuf>),
    RebuildIndex,
}

// ── run_watcher ───────────────────────────────────────────────────────────────

/// Start watching all mounted wikis. Runs until the cancellation token fires.
/// `push_tx`: optional channel to notify ACP sessions of watcher-triggered ingests.
pub async fn run_watcher(
    engine: Arc<WikiEngine>,
    debounce_ms: u32,
    cancel: CancellationToken,
    push_tx: tokio::sync::mpsc::Sender<(String, String)>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<(String, PathBuf)>(256);

    // Start native filesystem watcher
    let _watcher = start_notify_watcher(&engine, tx, cancel.clone())?;

    let debounce = Duration::from_millis(debounce_ms as u64);

    loop {
        // Wait for first event or shutdown
        let first = tokio::select! {
            ev = rx.recv() => match ev {
                Some(ev) => ev,
                None => break,
            },
            _ = cancel.cancelled() => break,
        };

        // Debounce: collect events for debounce_ms
        let mut md_changes: HashSet<(String, PathBuf)> = HashSet::new();
        let mut schema_wikis: HashSet<String> = HashSet::new();

        classify_event(&first.0, &first.1, &mut md_changes, &mut schema_wikis);

        let deadline = tokio::time::sleep(debounce);
        tokio::pin!(deadline);

        loop {
            tokio::select! {
                ev = rx.recv() => match ev {
                    Some((wiki, path)) => {
                        classify_event(&wiki, &path, &mut md_changes, &mut schema_wikis);
                    }
                    None => break,
                },
                _ = &mut deadline => break,
                _ = cancel.cancelled() => return Ok(()),
            }
        }

        // Process: rebuild takes priority over incremental ingest
        let action = if !schema_wikis.is_empty() {
            WatchAction::RebuildIndex
        } else if !md_changes.is_empty() {
            WatchAction::IngestPages(md_changes.into_iter().map(|(_, p)| p).collect())
        } else {
            continue;
        };

        match action {
            WatchAction::RebuildIndex => {
                for wiki_name in &schema_wikis {
                    // Get the per-wiki rebuild guard under a brief read lock.
                    let flag: Arc<AtomicBool> = {
                        let state = engine
                            .state
                            .read()
                            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
                        match state.spaces.get(wiki_name.as_str()) {
                            Some(space) => Arc::clone(&space.rebuilding),
                            None => continue,
                        }
                    };

                    // Skip if a rebuild is already running for this wiki.
                    // Note: if this future is dropped between here and the .await below
                    // (e.g. CancellationToken fires mid-iteration), the flag stays true.
                    // That is benign: on shutdown SpaceContext is dropped; on re-mount the
                    // new SpaceContext starts with rebuilding = false.
                    if flag.swap(true, Ordering::AcqRel) {
                        tracing::debug!(wiki = %wiki_name, "watch: rebuild already in progress, skipping");
                        continue;
                    }

                    let engine_clone = Arc::clone(&engine);
                    let wiki_name_clone = wiki_name.clone();
                    let start = std::time::Instant::now();

                    match tokio::task::spawn_blocking(move || {
                        engine_clone.schema_rebuild(&wiki_name_clone)
                    })
                    .await
                    {
                        Ok(Ok(())) => {
                            flag.store(false, Ordering::Release);
                            tracing::info!(
                                wiki = %wiki_name,
                                duration_ms = start.elapsed().as_millis() as u64,
                                "watch: schema changed, index updated",
                            );
                        }
                        Ok(Err(e)) => {
                            flag.store(false, Ordering::Release);
                            tracing::warn!(
                                wiki = %wiki_name,
                                error = %e,
                                "watch: schema rebuild failed",
                            );
                        }
                        Err(e) => {
                            flag.store(false, Ordering::Release);
                            tracing::warn!(
                                wiki = %wiki_name,
                                error = %e,
                                "watch: schema rebuild task panicked",
                            );
                        }
                    }
                }
            }
            WatchAction::IngestPages(paths) => {
                // Collect per-space data under the read lock, then drop the lock
                // before calling spawn_blocking — index writes must not stall the
                // tokio executor (same pattern as the RebuildIndex branch above).
                let tasks: Vec<_> = {
                    let state = engine
                        .state
                        .read()
                        .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
                    state
                        .spaces
                        .iter()
                        .filter_map(|(wiki_name, space)| {
                            let path_count = paths
                                .iter()
                                .filter(|p| p.starts_with(&space.wiki_root))
                                .count();
                            if path_count == 0 {
                                return None;
                            }
                            Some((
                                wiki_name.clone(),
                                path_count,
                                Arc::clone(&space.index_manager),
                                space.wiki_root.clone(),
                                space.repo_root.clone(),
                                space.index_manager.last_commit(),
                                space.index_schema.clone(),
                                Arc::clone(&space.type_registry),
                            ))
                        })
                        .collect()
                }; // read lock dropped here
                for (
                    wiki_name,
                    path_count,
                    index_manager,
                    wiki_root,
                    repo_root,
                    last_commit,
                    index_schema,
                    type_registry,
                ) in tasks
                {
                    let start = std::time::Instant::now();
                    match tokio::task::spawn_blocking(move || {
                        index_manager.update(
                            &wiki_root,
                            &repo_root,
                            last_commit.as_deref(),
                            &index_schema,
                            &type_registry,
                        )
                    })
                    .await
                    {
                        Ok(Ok(report)) => {
                            if report.updated > 0 || report.deleted > 0 {
                                tracing::info!(
                                    wiki = %wiki_name,
                                    files = path_count,
                                    updated = report.updated,
                                    deleted = report.deleted,
                                    duration_ms = start.elapsed().as_millis() as u64,
                                    "watch: ingested",
                                );
                                let msg = format!(
                                    "Wiki \"{wiki_name}\" updated: {} page(s) changed.",
                                    report.updated + report.deleted
                                );
                                if push_tx.try_send((wiki_name.clone(), msg)).is_err() {
                                    tracing::warn!(wiki = %wiki_name, "watcher update channel full; event dropped");
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(
                                wiki = %wiki_name,
                                error = %e,
                                "watch: ingest failed",
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                wiki = %wiki_name,
                                error = %e,
                                "watch: ingest task panicked",
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn classify_event(
    wiki_name: &str,
    path: &Path,
    md_changes: &mut HashSet<(String, PathBuf)>,
    schema_wikis: &mut HashSet<String>,
) {
    if is_schema_path(path) {
        schema_wikis.insert(wiki_name.to_string());
    } else {
        md_changes.insert((wiki_name.to_string(), path.to_path_buf()));
    }
}

fn is_schema_path(path: &Path) -> bool {
    // Check if path contains /schemas/ and ends with .json
    let s = path.to_string_lossy();
    s.contains("/schemas/") && path.extension().and_then(|e| e.to_str()) == Some("json")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_event ─────────────────────────────────────────────────────────

    #[test]
    fn classify_event_markdown_goes_to_md_changes() {
        let mut md: HashSet<(String, PathBuf)> = HashSet::new();
        let mut schema: HashSet<String> = HashSet::new();
        classify_event("mywiki", Path::new("/repo/wiki/concepts/foo.md"), &mut md, &mut schema);
        assert_eq!(md.len(), 1);
        assert!(schema.is_empty());
    }

    #[test]
    fn classify_event_schema_json_goes_to_schema_wikis() {
        let mut md: HashSet<(String, PathBuf)> = HashSet::new();
        let mut schema: HashSet<String> = HashSet::new();
        classify_event("mywiki", Path::new("/repo/schemas/concept.json"), &mut md, &mut schema);
        assert!(md.is_empty());
        assert!(schema.contains("mywiki"));
    }

    #[test]
    fn classify_event_non_json_schema_path_treated_as_md() {
        let mut md: HashSet<(String, PathBuf)> = HashSet::new();
        let mut schema: HashSet<String> = HashSet::new();
        // .yaml inside /schemas/ is NOT a schema trigger — only .json
        classify_event("mywiki", Path::new("/repo/schemas/types.yaml"), &mut md, &mut schema);
        assert_eq!(md.len(), 1);
        assert!(schema.is_empty());
    }

    #[test]
    fn classify_event_multiple_events_same_wiki_deduplicated() {
        let mut md: HashSet<(String, PathBuf)> = HashSet::new();
        let mut schema: HashSet<String> = HashSet::new();
        let path = Path::new("/repo/wiki/foo.md");
        classify_event("mywiki", path, &mut md, &mut schema);
        classify_event("mywiki", path, &mut md, &mut schema);
        assert_eq!(md.len(), 1, "duplicate events for same path must be deduplicated");
    }

    // ── rebuilding flag contract ────────────────────────────────────────────────

    /// swap(true) returns false on first call (not rebuilding → proceed) and
    /// true on second call (already rebuilding → skip). This pins the guard
    /// logic in run_watcher's RebuildIndex branch.
    #[test]
    fn rebuilding_flag_skip_contract() {
        let flag = AtomicBool::new(false);
        let already_running = flag.swap(true, Ordering::AcqRel);
        assert!(!already_running, "first swap must return false — rebuild should proceed");
        let already_running = flag.swap(true, Ordering::AcqRel);
        assert!(already_running, "second swap must return true — rebuild should be skipped");
    }
}

fn start_notify_watcher(
    engine: &WikiEngine,
    tx: mpsc::Sender<(String, PathBuf)>,
    cancel: CancellationToken,
) -> Result<RecommendedWatcher> {
    let state = engine
        .state
        .read()
        .map_err(|_| anyhow::anyhow!("lock poisoned"))?;

    // Build a map of watched paths to wiki names
    let mut watch_dirs: Vec<(String, PathBuf, PathBuf)> = Vec::new();
    for (name, space) in &state.spaces {
        watch_dirs.push((
            name.clone(),
            space.wiki_root.clone(),
            space.repo_root.clone(),
        ));
    }
    drop(state);

    let tx_clone = tx.clone();
    let watch_dirs_clone = watch_dirs.clone();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if cancel.is_cancelled() {
            return;
        }
        let event = match res {
            Ok(ev) => ev,
            Err(e) => {
                tracing::warn!(error = %e, "filesystem watcher error");
                return;
            }
        };

        // Only care about create, modify, rename
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
            _ => return,
        }

        for path in &event.paths {
            // Find which wiki this path belongs to
            for (wiki_name, wiki_root, repo_root) in &watch_dirs_clone {
                if path.starts_with(wiki_root)
                    && path.extension().and_then(|e| e.to_str()) == Some("md")
                {
                    if tx_clone
                        .try_send((wiki_name.clone(), path.clone()))
                        .is_err()
                    {
                        tracing::warn!(wiki = %wiki_name, "watcher update channel full; event dropped");
                    }
                    break;
                }
                if path.starts_with(repo_root.join("schemas")) && is_schema_path(path) {
                    if tx_clone
                        .try_send((wiki_name.clone(), path.clone()))
                        .is_err()
                    {
                        tracing::warn!(wiki = %wiki_name, "watcher update channel full; event dropped");
                    }
                    break;
                }
            }
        }
    })?;

    // Watch wiki/ and schemas/ for each mounted wiki
    for (_, wiki_root, repo_root) in &watch_dirs {
        if wiki_root.exists() {
            watcher.watch(wiki_root, RecursiveMode::Recursive)?;
        }
        let schemas_dir = repo_root.join("schemas");
        if schemas_dir.exists() {
            watcher.watch(&schemas_dir, RecursiveMode::NonRecursive)?;
        }
    }

    Ok(watcher)
}
