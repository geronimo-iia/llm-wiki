use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{bail, Result};

use crate::config::{self, GlobalConfig, ResolvedConfig};
use crate::index_schema::IndexSchema;
use crate::indexing;
use crate::space_builder;
use crate::type_registry::SpaceTypeRegistry;

// ── SpaceState ────────────────────────────────────────────────────────────────

pub struct SpaceState {
    pub name: String,
    pub wiki_root: PathBuf,
    pub repo_root: PathBuf,
    pub index_path: PathBuf,
    pub type_registry: SpaceTypeRegistry,
    pub index_schema: IndexSchema,
}

impl SpaceState {
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
    pub spaces: HashMap<String, SpaceState>,
}

impl Engine {
    pub fn default_wiki_name(&self) -> &str {
        &self.config.global.default_wiki
    }

    pub fn space(&self, name: &str) -> Result<&SpaceState> {
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

            // Ensure index directory exists
            let search_dir = index_path.join("search-index");
            std::fs::create_dir_all(&search_dir)?;

            // Check staleness and rebuild if needed
            let status = indexing::index_status(&entry.name, &index_path, &repo_root);
            let needs_first_build = status.as_ref().map(|s| s.built.is_none()).unwrap_or(true);

            if needs_first_build {
                tracing::info!(wiki = %entry.name, "building index for the first time");
                if let Err(e) = indexing::rebuild_index(
                    &wiki_root, &index_path, &entry.name, &repo_root,
                    &index_schema, &type_registry,
                ) {
                    tracing::warn!(wiki = %entry.name, error = %e, "initial index build failed");
                }
            } else if let Ok(ref s) = status {
                if s.stale && config.index.auto_rebuild {
                    tracing::info!(wiki = %entry.name, "index stale, rebuilding");
                    if let Err(e) = indexing::rebuild_index(
                        &wiki_root, &index_path, &entry.name, &repo_root,
                        &index_schema, &type_registry,
                    ) {
                        tracing::warn!(wiki = %entry.name, error = %e, "index rebuild failed");
                    }
                } else if s.stale {
                    tracing::warn!(
                        wiki = %entry.name,
                        "index stale — run `llm-wiki index rebuild --wiki {}`",
                        entry.name,
                    );
                }
            }

            spaces.insert(
                entry.name.clone(),
                SpaceState {
                    name: entry.name.clone(),
                    wiki_root,
                    repo_root,
                    index_path,
                    type_registry,
                    index_schema,
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

    pub fn on_ingest(&self, wiki_name: &str) -> Result<indexing::UpdateReport> {
        let engine = self
            .engine
            .read()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let space = engine.space(wiki_name)?;
        let last_commit = indexing::last_indexed_commit(&space.index_path);
        indexing::update_index(
            &space.wiki_root,
            &space.index_path,
            &space.repo_root,
            last_commit.as_deref(),
            &space.index_schema,
            wiki_name,
            &space.type_registry,
        )
    }

    pub fn rebuild_index(&self, wiki_name: &str) -> Result<indexing::IndexReport> {
        let engine = self
            .engine
            .read()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let space = engine.space(wiki_name)?;
        indexing::rebuild_index(
            &space.wiki_root,
            &space.index_path,
            wiki_name,
            &space.repo_root,
            &space.index_schema,
            &space.type_registry,
        )
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
