use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
                    let start = std::time::Instant::now();
                    match engine.schema_rebuild(wiki_name) {
                        Ok(()) => {
                            tracing::info!(
                                wiki = %wiki_name,
                                duration_ms = start.elapsed().as_millis() as u64,
                                "watch: schema changed, index updated",
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                wiki = %wiki_name,
                                error = %e,
                                "watch: schema rebuild failed",
                            );
                        }
                    }
                }
            }
            WatchAction::IngestPages(paths) => {
                // Group by wiki
                let state = engine
                    .state
                    .read()
                    .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
                for (wiki_name, space) in &state.spaces {
                    let wiki_paths: Vec<&PathBuf> = paths
                        .iter()
                        .filter(|p| p.starts_with(&space.wiki_root))
                        .collect();
                    if wiki_paths.is_empty() {
                        continue;
                    }
                    let start = std::time::Instant::now();
                    let last_commit = space.index_manager.last_commit();
                    match space.index_manager.update(
                        &space.wiki_root,
                        &space.repo_root,
                        last_commit.as_deref(),
                        &space.index_schema,
                        &space.type_registry,
                    ) {
                        Ok(report) => {
                            if report.updated > 0 || report.deleted > 0 {
                                tracing::info!(
                                    wiki = %wiki_name,
                                    files = wiki_paths.len(),
                                    updated = report.updated,
                                    deleted = report.deleted,
                                    duration_ms = start.elapsed().as_millis() as u64,
                                    "watch: ingested",
                                );
                                let msg = format!(
                                    "Wiki \"{wiki_name}\" updated: {} page(s) changed.",
                                    report.updated + report.deleted
                                );
                                let _ = push_tx.try_send((wiki_name.clone(), msg));
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                wiki = %wiki_name,
                                error = %e,
                                "watch: ingest failed",
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

fn is_wiki_md(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("/wiki/") && path.extension().and_then(|e| e.to_str()) == Some("md")
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
            Err(_) => return,
        };

        // Only care about create, modify, rename
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
            _ => return,
        }

        for path in &event.paths {
            // Find which wiki this path belongs to
            for (wiki_name, wiki_root, repo_root) in &watch_dirs_clone {
                if path.starts_with(wiki_root) && is_wiki_md(path) {
                    let _ = tx_clone.try_send((wiki_name.clone(), path.clone()));
                    break;
                }
                if path.starts_with(repo_root.join("schemas")) && is_schema_path(path) {
                    let _ = tx_clone.try_send((wiki_name.clone(), path.clone()));
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
