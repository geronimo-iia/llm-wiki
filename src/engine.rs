use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{bail, Result};

use crate::config::{self, GlobalConfig, ResolvedConfig};
use crate::index_manager::{IndexReport, SpaceIndexManager, UpdateReport};
use crate::index_schema::IndexSchema;
use crate::space_builder;
use crate::type_registry::SpaceTypeRegistry;

// ── SpaceContext ──────────────────────────────────────────────────────────────

pub struct SpaceContext {
    pub name: String,
    pub wiki_root: PathBuf,
    pub repo_root: PathBuf,
    pub type_registry: SpaceTypeRegistry,
    pub index_schema: IndexSchema,
    pub index_manager: SpaceIndexManager,
}

impl SpaceContext {
    pub fn resolved_config(&self, global: &GlobalConfig) -> ResolvedConfig {
        let wiki_cfg = config::load_wiki(&self.repo_root).unwrap_or_default();
        config::resolve(global, &wiki_cfg)
    }
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct Engine {
    pub config: GlobalConfig,
    pub config_path: PathBuf,
    pub state_dir: PathBuf,
    pub spaces: HashMap<String, SpaceContext>,
}

impl Engine {
    pub fn default_wiki_name(&self) -> &str {
        &self.config.global.default_wiki
    }

    pub fn space(&self, name: &str) -> Result<&SpaceContext> {
        self.spaces
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("wiki \"{name}\" is not mounted"))
    }

    pub fn resolve_wiki_name<'a>(&'a self, explicit: Option<&'a str>) -> &'a str {
        explicit.unwrap_or(self.default_wiki_name())
    }

    pub fn index_path_for(&self, wiki_name: &str) -> PathBuf {
        self.state_dir.join("indexes").join(wiki_name)
    }
}

// ── EngineManager ─────────────────────────────────────────────────────────────

pub struct EngineManager {
    pub engine: Arc<RwLock<Engine>>,
}

impl EngineManager {
    pub fn build(config_path: &Path) -> Result<Self> {
        let config = config::load_global(config_path)?;
        let state_dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let mut spaces = HashMap::new();

        for entry in &config.wikis {
            let repo_root = PathBuf::from(&entry.path);
            let wiki_root = repo_root.join("wiki");
            let index_path = state_dir.join("indexes").join(&entry.name);

            // Build per-wiki registry + index schema from schemas/
            let (type_registry, index_schema) =
                space_builder::build_space(&repo_root, &config.index.tokenizer)
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            wiki = %entry.name, error = %e,
                            "failed to build type registry, using embedded defaults"
                        );
                        space_builder::build_space_from_embedded(&config.index.tokenizer)
                    });

            let mut index_manager = SpaceIndexManager::new(&entry.name, &index_path);

            // Ensure index directory exists
            let search_dir = index_path.join("search-index");
            std::fs::create_dir_all(&search_dir)?;

            // Check staleness and rebuild if needed
            let status = index_manager.status(&repo_root);
            let needs_first_build = status.as_ref().map(|s| s.built.is_none()).unwrap_or(true);

            if needs_first_build {
                tracing::info!(wiki = %entry.name, "building index for the first time");
                if let Err(e) = index_manager.rebuild(
                    &wiki_root, &repo_root, &index_schema, &type_registry,
                ) {
                    tracing::warn!(wiki = %entry.name, error = %e, "initial index build failed");
                }
            } else if config.index.auto_rebuild {
                use crate::index_manager::StalenessKind;
                match index_manager.staleness_kind(&repo_root) {
                    Ok(StalenessKind::Current) => {}
                    Ok(StalenessKind::CommitChanged) => {
                        tracing::info!(wiki = %entry.name, "index behind HEAD, updating");
                        let last = index_manager.last_commit();
                        if let Err(e) = index_manager.update(
                            &wiki_root, &repo_root, last.as_deref(),
                            &index_schema, &type_registry,
                        ) {
                            tracing::warn!(wiki = %entry.name, error = %e, "incremental update failed");
                        }
                    }
                    Ok(StalenessKind::TypesChanged(types)) => {
                        tracing::info!(wiki = %entry.name, types = ?types, "types changed, partial rebuild");
                        if let Err(e) = index_manager.rebuild_types(
                            &types, &wiki_root, &repo_root, &index_schema, &type_registry,
                        ) {
                            tracing::warn!(wiki = %entry.name, error = %e, "partial rebuild failed, doing full");
                            let _ = index_manager.rebuild(
                                &wiki_root, &repo_root, &index_schema, &type_registry,
                            );
                        }
                    }
                    Ok(StalenessKind::FullRebuildNeeded) => {
                        tracing::info!(wiki = %entry.name, "index stale, rebuilding");
                        if let Err(e) = index_manager.rebuild(
                            &wiki_root, &repo_root, &index_schema, &type_registry,
                        ) {
                            tracing::warn!(wiki = %entry.name, error = %e, "index rebuild failed");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(wiki = %entry.name, error = %e, "staleness check failed, rebuilding");
                        let _ = index_manager.rebuild(
                            &wiki_root, &repo_root, &index_schema, &type_registry,
                        );
                    }
                }
            } else if let Ok(ref s) = status {
                if s.stale {
                    tracing::warn!(
                        wiki = %entry.name,
                        "index stale — run `llm-wiki index rebuild --wiki {}`",
                        entry.name,
                    );
                }
            }

            // Open the index for serving (hold reader in memory)
            if let Err(e) = index_manager.open(
                &index_schema,
                Some((&wiki_root, &repo_root, &type_registry)),
            ) {
                tracing::warn!(wiki = %entry.name, error = %e, "failed to open index");
            }

            spaces.insert(
                entry.name.clone(),
                SpaceContext {
                    name: entry.name.clone(),
                    wiki_root,
                    repo_root,
                    type_registry,
                    index_schema,
                    index_manager,
                },
            );
        }

        let engine = Engine {
            config,
            config_path: config_path.to_path_buf(),
            state_dir,
            spaces,
        };

        Ok(EngineManager {
            engine: Arc::new(RwLock::new(engine)),
        })
    }

    pub fn on_ingest(&self, wiki_name: &str) -> Result<UpdateReport> {
        let engine = self
            .engine
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

    pub fn rebuild_index(&self, wiki_name: &str) -> Result<IndexReport> {
        let engine = self
            .engine
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

    pub fn on_wiki_added(&self, _name: &str, _path: &Path) -> Result<()> {
        bail!("wiki added — restart required")
    }

    pub fn on_wiki_removed(&self, _name: &str) -> Result<()> {
        bail!("wiki removed — restart required")
    }

    pub fn on_config_change(&self, _key: &str, _value: &str) -> Result<()> {
        bail!("config changed — restart required")
    }
}

pub fn default_index_path_for(wiki_name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".llm-wiki")
        .join("indexes")
        .join(wiki_name)
}
