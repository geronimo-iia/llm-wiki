#![allow(unreachable_pub)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Context, Result};

use petgraph_live::cache::GenerationCache;
use petgraph_live::live::{GraphState, GraphStateConfig};
use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};

use crate::config::{self, GlobalConfig, IngestConfig, ResolvedConfig, WikiEntry};
use crate::graph::{CommunityData, WikiGraph, WikiGraphCache};
use crate::index_manager::{IndexReport, SpaceIndexManager, StalenessKind, UpdateReport};
use crate::index_schema::IndexSchema;
use crate::space_builder;
use crate::type_registry::SpaceTypeRegistry;

// ── SpaceContext ──────────────────────────────────────────────────────────────

/// All runtime state for a single mounted wiki space.
pub struct SpaceContext {
    /// Registered name of this wiki space.
    pub name: String,
    /// Absolute path to the `wiki/` subdirectory containing Markdown pages.
    pub wiki_root: PathBuf,
    /// Absolute path to the git repository root (parent of `wiki/`).
    pub repo_root: PathBuf,
    /// Type registry compiled from the wiki's schema files.
    pub type_registry: Arc<SpaceTypeRegistry>,
    /// Tantivy index schema for this space.
    pub index_schema: IndexSchema,
    /// Lifecycle manager for the Tantivy search index.
    pub index_manager: Arc<SpaceIndexManager>,
    /// Graph cache — either in-memory only (NoSnapshot) or snapshot-backed (WithSnapshot).
    /// `petgraph_live::GenerationCache` is internally `Send + Sync`; concurrent readers and
    /// a rebuilder are safe because `get_fresh` and `rebuild` operate on the cache's own
    /// internal `RwLock`. No external lock is required around `graph_cache` accesses.
    pub graph_cache: WikiGraphCache,
    /// Generation-keyed community cache. Shares the same generation key as graph_cache.
    pub community_cache: GenerationCache<CommunityData>,
    /// Guard preventing redundant concurrent rebuilds. Set to `true` while a
    /// rebuild is in progress; watch.rs checks before dispatching a new one.
    /// If run_watcher is cancelled between setting and clearing this flag,
    /// the engine is shutting down and SpaceContext will be dropped anyway.
    pub rebuilding: Arc<AtomicBool>,
    /// Resolved ingest configuration for this wiki — used by rebuild and update calls.
    pub ingest_config: IngestConfig,
}

impl SpaceContext {
    /// Load and resolve the per-wiki config merged with `global`.
    pub fn resolved_config(&self, global: &GlobalConfig) -> ResolvedConfig {
        let wiki_cfg = config::load_wiki(&self.repo_root).unwrap_or_else(|e| {
            tracing::warn!(path = %self.repo_root.display(), error = %e, "failed to load wiki config, using defaults");
            Default::default()
        });
        config::resolve(global, &wiki_cfg)
    }
}

// ── EngineState ──────────────────────────────────────────────────────────────

/// Shared mutable state protected by [`WikiEngine`]'s `RwLock`.
pub struct EngineState {
    /// Loaded global configuration.
    pub config: GlobalConfig,
    /// Absolute path to the global config file on disk.
    pub config_path: PathBuf,
    /// Directory that holds per-wiki index state (parent of the config file).
    pub state_dir: PathBuf,
    /// Map from wiki name to its mounted `SpaceContext`.
    pub spaces: HashMap<String, Arc<SpaceContext>>,
}

impl EngineState {
    /// Return the configured default wiki name, or `None` if unset.
    pub fn default_wiki_name(&self) -> Option<&str> {
        self.config.global.default_wiki_opt()
    }

    /// Look up a mounted wiki space by name. Errors if not mounted.
    pub fn space(&self, name: &str) -> Result<&Arc<SpaceContext>> {
        self.spaces
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("wiki \"{name}\" is not mounted"))
    }

    /// Return `explicit` if given, otherwise the default wiki name, or an error.
    pub fn resolve_wiki_name<'a>(&'a self, explicit: Option<&'a str>) -> Result<&'a str> {
        explicit
            .or_else(|| self.default_wiki_name())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no wiki specified and no default wiki configured \u{2014} \
                     run `llm-wiki spaces set-default <name>`"
                )
            })
    }

    /// Return the index directory path for a wiki by name.
    pub fn index_path_for(&self, wiki_name: &str) -> PathBuf {
        self.state_dir.join("indexes").join(wiki_name)
    }
}

// ── WikiEngine ─────────────────────────────────────────────────────────────

/// Central engine — owns all wiki spaces and exposes index/mount operations.
///
/// Cheap to clone (`Arc` inside). Safe to share across async tasks.
pub struct WikiEngine {
    /// Shared engine state protected by a reader-writer lock.
    pub state: Arc<RwLock<EngineState>>,
    /// Serializes config file mutations (load → modify → save) to prevent lost writes.
    pub config_write_lock: Arc<Mutex<()>>,
}

impl WikiEngine {
    /// Build a `WikiEngine` from the global config at `config_path`, mounting all registered wikis.
    pub fn build(config_path: &Path) -> Result<Self> {
        let config = config::load_global(config_path)?;
        let state_dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let mut spaces = HashMap::new();
        let mut mount_failures = 0usize;

        for entry in &config.wikis {
            match mount_space(entry, &state_dir, &config) {
                Ok(ctx) => {
                    spaces.insert(entry.name.clone(), Arc::new(ctx));
                }
                Err(e) => {
                    mount_failures += 1;
                    tracing::warn!(
                        wiki = %entry.name, error = %format_args!("{e:#}"),
                        "failed to mount wiki, skipping",
                    );
                }
            }
        }
        if mount_failures > 0 {
            tracing::warn!(
                count = mount_failures,
                "failed to mount {} wiki(s); see prior messages for details",
                mount_failures
            );
        }

        let engine = EngineState {
            config,
            config_path: config_path.to_path_buf(),
            state_dir,
            spaces,
        };

        Ok(WikiEngine {
            state: Arc::new(RwLock::new(engine)),
            config_write_lock: Arc::new(Mutex::new(())),
        })
    }

    /// Incrementally update the index from git changes since the last indexed commit.
    pub fn refresh_index(&self, wiki_name: &str) -> Result<UpdateReport> {
        let space: Arc<SpaceContext> = {
            let engine = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
            Arc::clone(engine.space(wiki_name)?)
        };
        let last_commit = space.index_manager.last_commit();
        let report = space.index_manager.update(
            &space.wiki_root,
            &space.repo_root,
            last_commit.as_deref(),
            &space.index_schema,
            &space.type_registry,
            &space.ingest_config,
        )?;
        if report.updated > 0 || report.deleted > 0 {
            tracing::info!(
                wiki = %wiki_name,
                updated = report.updated,
                deleted = report.deleted,
                "index updated",
            );
        }
        Ok(report)
    }

    /// Rebuild the search index from scratch by walking the wiki tree.
    pub fn rebuild_index(&self, wiki_name: &str) -> Result<IndexReport> {
        let (space, rebuilding) = {
            let engine = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
            let sp = engine.space(wiki_name)?;
            (Arc::clone(sp), Arc::clone(&sp.rebuilding))
        };
        // Signal to the watcher that a rebuild is in progress so it doesn't
        // dispatch a competing one. rebuild_lock inside SpaceIndexManager serializes
        // any rebuild that slips through before this store.
        rebuilding.store(true, std::sync::atomic::Ordering::Release);
        let result = space.index_manager.rebuild(
            &space.wiki_root,
            &space.repo_root,
            &space.index_schema,
            &space.type_registry,
            &space.ingest_config,
        );
        rebuilding.store(false, std::sync::atomic::Ordering::Release);
        let report = result?;
        tracing::info!(
            wiki = %wiki_name,
            pages = report.pages_indexed,
            duration_ms = report.duration_ms,
            "index rebuilt",
        );
        Ok(report)
    }

    /// Smart schema rebuild: checks staleness and does partial rebuild
    /// when possible, full rebuild only when necessary.
    pub fn schema_rebuild(&self, wiki_name: &str) -> Result<()> {
        // Hold the read lock only long enough to clone the Arc — drops before I/O.
        let space: Arc<SpaceContext> = {
            let engine = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
            Arc::clone(engine.space(wiki_name)?)
        };
        match space.index_manager.staleness_kind(&space.repo_root) {
            Ok(StalenessKind::Current) => {}
            Ok(StalenessKind::CommitChanged) => {
                let last = space.index_manager.last_commit();
                space.index_manager.update(
                    &space.wiki_root,
                    &space.repo_root,
                    last.as_deref(),
                    &space.index_schema,
                    &space.type_registry,
                    &space.ingest_config,
                )?;
            }
            Ok(StalenessKind::TypesChanged(types)) => {
                tracing::info!(wiki = %wiki_name, types = ?types, "partial rebuild");
                if let Err(e) = space.index_manager.rebuild_types(
                    &types,
                    &space.wiki_root,
                    &space.repo_root,
                    &space.index_schema,
                    &space.type_registry,
                    &space.ingest_config,
                ) {
                    tracing::warn!(wiki = %wiki_name, error = %e, "partial rebuild failed, doing full");
                    space.index_manager.rebuild(
                        &space.wiki_root,
                        &space.repo_root,
                        &space.index_schema,
                        &space.type_registry,
                        &space.ingest_config,
                    )?;
                }
            }
            Ok(StalenessKind::FullRebuildNeeded) => {
                space.index_manager.rebuild(
                    &space.wiki_root,
                    &space.repo_root,
                    &space.index_schema,
                    &space.type_registry,
                    &space.ingest_config,
                )?;
            }
            Err(e) => {
                tracing::warn!(error = %e, "staleness check failed; falling back to full rebuild");
                space.index_manager.rebuild(
                    &space.wiki_root,
                    &space.repo_root,
                    &space.index_schema,
                    &space.type_registry,
                    &space.ingest_config,
                )?;
            }
        }
        Ok(())
    }

    /// Mount a wiki into the running engine. Called by space management
    /// tools for hot reload.
    pub fn mount_wiki(&self, entry: &WikiEntry) -> Result<()> {
        // Clone cheap fields under the read lock, then drop before I/O.
        let (state_dir, config) = {
            let engine = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
            (engine.state_dir.clone(), engine.config.clone())
        };
        let ctx = mount_space(entry, &state_dir, &config)?;
        let mut engine = self
            .state
            .write()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        engine.spaces.insert(entry.name.clone(), Arc::new(ctx));
        tracing::info!(wiki = %entry.name, "reload: mounted");
        Ok(())
    }

    /// Unmount a wiki from the running engine. Refuses if the wiki is
    /// the current default. In-flight requests holding an `Arc<SpaceContext>`
    /// complete normally.
    pub fn unmount_wiki(&self, name: &str) -> Result<()> {
        let mut engine = self
            .state
            .write()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        if engine.default_wiki_name() == Some(name) {
            anyhow::bail!("\"{name}\" is the default wiki \u{2014} set a new default first");
        }
        if engine.spaces.remove(name).is_none() {
            anyhow::bail!("wiki \"{name}\" is not mounted");
        }
        tracing::info!(wiki = %name, "reload: unmounted");
        Ok(())
    }

    /// Serialize a config mutation (load → modify → save) so concurrent MCP
    /// transports cannot interleave their read-modify-write cycles and lose writes.
    pub fn with_config_lock<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        let _guard = self
            .config_write_lock
            .lock()
            .map_err(|_| anyhow::anyhow!("config lock poisoned"))?;
        f()
    }

    /// Update the in-memory default wiki. The wiki must be mounted.
    ///
    /// Intentionally in-memory only — callers that need disk persistence must also call
    /// `spaces::set_default_wiki()`. `ops::spaces_set_default()` does both atomically
    /// under `with_config_lock`; do not call this directly from handlers.
    pub fn set_default(&self, name: &str) -> Result<()> {
        let mut engine = self
            .state
            .write()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        if !engine.spaces.contains_key(name) {
            anyhow::bail!("wiki \"{name}\" is not mounted");
        }
        engine.config.global.default_wiki = name.to_string();
        tracing::info!(wiki = %name, "reload: default updated");
        Ok(())
    }
}

// ── mount_wiki ────────────────────────────────────────────────────────────────

fn mount_space(entry: &WikiEntry, state_dir: &Path, config: &GlobalConfig) -> Result<SpaceContext> {
    let repo_root = crate::pathutil::strip_verbatim_prefix(entry.path.clone());
    let wiki_cfg = config::load_wiki(&repo_root).unwrap_or_default();
    let resolved_cfg = config::resolve(config, &wiki_cfg);
    let wiki_root = repo_root.join(&wiki_cfg.wiki_root);
    let index_path = state_dir.join("indexes").join(&entry.name);

    // A broken schemas/ directory is a hard error: silently falling back to
    // embedded defaults would index pages against the wrong schema. A wiki
    // with no schemas/ directory at all still works — build_space handles
    // that case internally by using the embedded defaults.
    let (type_registry, index_schema) =
        space_builder::build_space(&repo_root, config.index.tokenizer.as_str()).with_context(
            || {
                format!(
                    "failed to build type registry for wiki \"{}\" from {}",
                    entry.name,
                    repo_root.join("schemas").display()
                )
            },
        )?;

    let index_manager = Arc::new(SpaceIndexManager::new(
        &entry.name,
        &index_path,
        (config.index.memory_budget_mb as usize) * 1_000_000,
    ));

    let search_dir = index_path.join("search-index");
    std::fs::create_dir_all(&search_dir)?;

    // Staleness check and rebuild
    let status = index_manager.status(&repo_root);
    let needs_first_build = status.as_ref().map(|s| s.built.is_none()).unwrap_or(true);

    if needs_first_build {
        tracing::info!(wiki = %entry.name, "building index for the first time");
        if let Err(e) = index_manager.rebuild(
            &wiki_root,
            &repo_root,
            &index_schema,
            &type_registry,
            &resolved_cfg.ingest,
        ) {
            tracing::error!(wiki = %entry.name, error = %e, "initial index build failed; wiki will serve no results");
        }
    } else if config.index.auto_rebuild {
        match index_manager.staleness_kind(&repo_root) {
            Ok(StalenessKind::Current) => {}
            Ok(StalenessKind::CommitChanged) => {
                tracing::info!(wiki = %entry.name, "index behind HEAD, updating");
                let last = index_manager.last_commit();
                if let Err(e) = index_manager.update(
                    &wiki_root,
                    &repo_root,
                    last.as_deref(),
                    &index_schema,
                    &type_registry,
                    &resolved_cfg.ingest,
                ) {
                    // warn not error: the watcher will retry on the next commit
                    tracing::warn!(wiki = %entry.name, error = %e, "incremental update failed; index serves last successful state");
                }
            }
            Ok(StalenessKind::TypesChanged(types)) => {
                tracing::info!(wiki = %entry.name, types = ?types, "types changed, partial rebuild");
                if let Err(e) = index_manager.rebuild_types(
                    &types,
                    &wiki_root,
                    &repo_root,
                    &index_schema,
                    &type_registry,
                    &resolved_cfg.ingest,
                ) {
                    tracing::warn!(wiki = %entry.name, error = %e, "partial rebuild failed, attempting full rebuild");
                    if let Err(e) = index_manager.rebuild(
                        &wiki_root,
                        &repo_root,
                        &index_schema,
                        &type_registry,
                        &resolved_cfg.ingest,
                    ) {
                        tracing::error!(wiki = %entry.name, error = %e, "full rebuild after partial failure also failed; wiki will serve stale results");
                    }
                }
            }
            Ok(StalenessKind::FullRebuildNeeded) => {
                tracing::info!(wiki = %entry.name, "index stale, rebuilding");
                if let Err(e) = index_manager.rebuild(
                    &wiki_root,
                    &repo_root,
                    &index_schema,
                    &type_registry,
                    &resolved_cfg.ingest,
                ) {
                    tracing::error!(wiki = %entry.name, error = %e, "index rebuild failed; wiki will serve stale results");
                }
            }
            Err(e) => {
                tracing::warn!(wiki = %entry.name, error = %e, "staleness check failed, attempting rebuild");
                if let Err(e) = index_manager.rebuild(
                    &wiki_root,
                    &repo_root,
                    &index_schema,
                    &type_registry,
                    &resolved_cfg.ingest,
                ) {
                    tracing::error!(wiki = %entry.name, error = %e, "rebuild after staleness check failure also failed; wiki will serve stale results");
                }
            }
        }
    } else if let Ok(ref s) = status
        && s.stale
    {
        tracing::warn!(
            wiki = %entry.name,
            "index stale — run `llm-wiki index rebuild --wiki {}`",
            entry.name,
        );
    }

    // Open the index for serving; pass recovery args only when auto_recovery is enabled.
    let recovery = if config.index.auto_recovery {
        Some((
            &wiki_root as &std::path::Path,
            &repo_root as &std::path::Path,
            &type_registry,
            &resolved_cfg.ingest,
        ))
    } else {
        None
    };
    if let Err(e) = index_manager.open(&index_schema, recovery) {
        tracing::error!(wiki = %entry.name, error = %e, "failed to open index; wiki will serve no results");
    }

    let type_registry = Arc::new(type_registry);
    let graph_cache = {
        let im_key = index_manager.clone();
        let im_build = index_manager.clone();
        let is = index_schema.clone();
        let tr = Arc::clone(&type_registry);
        build_wiki_graph_cache(
            &entry.name,
            state_dir,
            &resolved_cfg.graph,
            move || {
                Ok(im_key
                    .last_commit()
                    .unwrap_or_else(|| "no-commit".to_string()))
            },
            move || {
                let searcher = im_build.searcher().map_err(|e| {
                    petgraph_live::snapshot::SnapshotError::Io(std::io::Error::other(e.to_string()))
                })?;
                crate::graph::build_graph(
                    &searcher,
                    &is,
                    &crate::graph::GraphFilter::default(),
                    &tr,
                )
                .map_err(|e| {
                    petgraph_live::snapshot::SnapshotError::Io(std::io::Error::other(e.to_string()))
                })
            },
        )?
    };

    Ok(SpaceContext {
        name: entry.name.clone(),
        wiki_root,
        repo_root,
        type_registry,
        index_schema,
        index_manager,
        graph_cache,
        community_cache: GenerationCache::new(),
        rebuilding: Arc::new(AtomicBool::new(false)),
        ingest_config: resolved_cfg.ingest.clone(),
    })
}

fn build_wiki_graph_cache(
    wiki_name: &str,
    state_dir: &Path,
    graph_cfg: &crate::config::GraphConfig,
    key_fn: impl Fn() -> Result<String, petgraph_live::snapshot::SnapshotError> + Send + Sync + 'static,
    build_fn: impl Fn() -> Result<WikiGraph, petgraph_live::snapshot::SnapshotError>
    + Send
    + Sync
    + 'static,
) -> Result<WikiGraphCache> {
    if !graph_cfg.snapshot {
        return Ok(WikiGraphCache::NoSnapshot(GenerationCache::new()));
    }

    let compression = match graph_cfg.snapshot_format.as_str() {
        "bincode+lz4" => Compression::Lz4,
        "bincode+zstd" => Compression::Zstd { level: 3 },
        _ => {
            tracing::warn!(
                format = %graph_cfg.snapshot_format,
                "unknown snapshot_format value — falling back to uncompressed; valid: \"bincode+lz4\", \"bincode+zstd\", \"bincode\""
            );
            Compression::None
        }
    };

    let snap_cfg = SnapshotConfig {
        dir: state_dir.join("snapshots").join(wiki_name),
        name: "wiki-graph".into(),
        key: None,
        format: SnapshotFormat::Bincode,
        compression,
        keep: graph_cfg.snapshot_keep as usize,
    };

    let state = GraphState::builder(GraphStateConfig::new(snap_cfg))
        .key_fn(key_fn)
        .build_fn(build_fn)
        .init()
        .map_err(|e| anyhow::anyhow!("graph snapshot init failed: {e}"))?;

    Ok(WikiGraphCache::WithSnapshot(state))
}
