use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::Result;

use petgraph_live::cache::GenerationCache;
use petgraph_live::live::{GraphState, GraphStateConfig};
use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};

use crate::config::{self, GlobalConfig, ResolvedConfig, WikiEntry};
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
    pub graph_cache: WikiGraphCache,
    /// Generation-keyed community cache. Shares the same generation key as graph_cache.
    pub community_cache: GenerationCache<CommunityData>,
}

impl SpaceContext {
    /// Load and resolve the per-wiki config merged with `global`.
    pub fn resolved_config(&self, global: &GlobalConfig) -> ResolvedConfig {
        let wiki_cfg = config::load_wiki(&self.repo_root).unwrap_or_default();
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
    /// Return the configured default wiki name.
    pub fn default_wiki_name(&self) -> &str {
        &self.config.global.default_wiki
    }

    /// Look up a mounted wiki space by name. Errors if not mounted.
    pub fn space(&self, name: &str) -> Result<&Arc<SpaceContext>> {
        self.spaces
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("wiki \"{name}\" is not mounted"))
    }

    /// Return `explicit` if given, otherwise the default wiki name.
    pub fn resolve_wiki_name<'a>(&'a self, explicit: Option<&'a str>) -> &'a str {
        explicit.unwrap_or(self.default_wiki_name())
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
}

impl WikiEngine {
    /// Build a `WikiEngine` from the global config at `config_path`, mounting all registered wikis.
    pub fn build(config_path: &Path) -> Result<Self> {
        let config = config::load_global(config_path)?;
        let state_dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let mut spaces = HashMap::new();

        for entry in &config.wikis {
            match mount_space(entry, &state_dir, &config) {
                Ok(ctx) => {
                    spaces.insert(entry.name.clone(), Arc::new(ctx));
                }
                Err(e) => {
                    tracing::warn!(
                        wiki = %entry.name, error = %e,
                        "failed to mount wiki, skipping",
                    );
                }
            }
        }

        let engine = EngineState {
            config,
            config_path: config_path.to_path_buf(),
            state_dir,
            spaces,
        };

        Ok(WikiEngine {
            state: Arc::new(RwLock::new(engine)),
        })
    }

    /// Incrementally update the index from git changes since the last indexed commit.
    pub fn refresh_index(&self, wiki_name: &str) -> Result<UpdateReport> {
        let engine = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let space = engine.space(wiki_name)?;
        let last_commit = space.index_manager.last_commit();
        let report = space.index_manager.update(
            &space.wiki_root,
            &space.repo_root,
            last_commit.as_deref(),
            &space.index_schema,
            &space.type_registry,
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
        let engine = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let space = engine.space(wiki_name)?;
        let report = space.index_manager.rebuild(
            &space.wiki_root,
            &space.repo_root,
            &space.index_schema,
            &space.type_registry,
        )?;
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
        let engine = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let space = engine.space(wiki_name)?;
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
                ) {
                    tracing::warn!(wiki = %wiki_name, error = %e, "partial rebuild failed, doing full");
                    space.index_manager.rebuild(
                        &space.wiki_root,
                        &space.repo_root,
                        &space.index_schema,
                        &space.type_registry,
                    )?;
                }
            }
            Ok(StalenessKind::FullRebuildNeeded) | Err(_) => {
                space.index_manager.rebuild(
                    &space.wiki_root,
                    &space.repo_root,
                    &space.index_schema,
                    &space.type_registry,
                )?;
            }
        }
        Ok(())
    }

    /// Mount a wiki into the running engine. Called by space management
    /// tools for hot reload.
    pub fn mount_wiki(&self, entry: &WikiEntry) -> Result<()> {
        let mut engine = self
            .state
            .write()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let ctx = mount_space(entry, &engine.state_dir, &engine.config)?;
        tracing::info!(wiki = %entry.name, "reload: mounted");
        engine.spaces.insert(entry.name.clone(), Arc::new(ctx));
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
        if engine.default_wiki_name() == name {
            anyhow::bail!("\"{name}\" is the default wiki \u{2014} set a new default first");
        }
        if engine.spaces.remove(name).is_none() {
            anyhow::bail!("wiki \"{name}\" is not mounted");
        }
        tracing::info!(wiki = %name, "reload: unmounted");
        Ok(())
    }

    /// Update the default wiki. The wiki must be mounted.
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
    let repo_root = PathBuf::from(&entry.path);
    let wiki_cfg = config::load_wiki(&repo_root).unwrap_or_default();
    let wiki_root = repo_root.join(&wiki_cfg.wiki_root);
    let index_path = state_dir.join("indexes").join(&entry.name);

    let (type_registry, index_schema) =
        space_builder::build_space(&repo_root, &config.index.tokenizer).unwrap_or_else(|e| {
            tracing::warn!(
                wiki = %entry.name, error = %e,
                "failed to build type registry, using embedded defaults"
            );
            space_builder::build_space_from_embedded(&config.index.tokenizer)
        });

    let index_manager = Arc::new(SpaceIndexManager::new(&entry.name, &index_path));

    let search_dir = index_path.join("search-index");
    std::fs::create_dir_all(&search_dir)?;

    // Staleness check and rebuild
    let status = index_manager.status(&repo_root);
    let needs_first_build = status.as_ref().map(|s| s.built.is_none()).unwrap_or(true);

    if needs_first_build {
        tracing::info!(wiki = %entry.name, "building index for the first time");
        if let Err(e) = index_manager.rebuild(&wiki_root, &repo_root, &index_schema, &type_registry)
        {
            tracing::warn!(wiki = %entry.name, error = %e, "initial index build failed");
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
                ) {
                    tracing::warn!(wiki = %entry.name, error = %e, "incremental update failed");
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
                ) {
                    tracing::warn!(wiki = %entry.name, error = %e, "partial rebuild failed, doing full");
                    let _ = index_manager.rebuild(
                        &wiki_root,
                        &repo_root,
                        &index_schema,
                        &type_registry,
                    );
                }
            }
            Ok(StalenessKind::FullRebuildNeeded) => {
                tracing::info!(wiki = %entry.name, "index stale, rebuilding");
                if let Err(e) =
                    index_manager.rebuild(&wiki_root, &repo_root, &index_schema, &type_registry)
                {
                    tracing::warn!(wiki = %entry.name, error = %e, "index rebuild failed");
                }
            }
            Err(e) => {
                tracing::warn!(wiki = %entry.name, error = %e, "staleness check failed, rebuilding");
                let _ =
                    index_manager.rebuild(&wiki_root, &repo_root, &index_schema, &type_registry);
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

    // Open the index for serving
    if let Err(e) = index_manager.open(
        &index_schema,
        Some((&wiki_root, &repo_root, &type_registry)),
    ) {
        tracing::warn!(wiki = %entry.name, error = %e, "failed to open index");
    }

    let resolved_cfg = config::resolve(config, &wiki_cfg);
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
            move || Ok(im_key.generation().to_string()),
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
        _ => Compression::None,
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
