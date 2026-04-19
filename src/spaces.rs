use std::path::Path;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::config::{load_global, save_global, GlobalConfig, WikiEntry};
use crate::default_schemas::default_schemas;
use crate::git;

// ── CreateReport ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateReport {
    pub path: String,
    pub name: String,
    pub created: bool,
    pub registered: bool,
    pub committed: bool,
}

// ── create ────────────────────────────────────────────────────────────────────

pub fn create(
    path: &Path,
    name: &str,
    description: Option<&str>,
    force: bool,
    set_default: bool,
    config_path: &Path,
) -> Result<CreateReport> {
    let mut created = false;
    if !path.exists() {
        std::fs::create_dir_all(path)?;
        created = true;
    }
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut committed = false;

    // Check re-run conditions
    let global = load_global(config_path)?;
    if let Some(existing) = global
        .wikis
        .iter()
        .find(|w| w.path == path.to_string_lossy())
    {
        if existing.name == name {
            ensure_structure(&path, name, description)?;
            return Ok(CreateReport {
                path: path.to_string_lossy().into(),
                name: name.into(),
                created: false,
                registered: false,
                committed: false,
            });
        } else if !force {
            bail!(
                "wiki already registered as \"{}\". Use --force to rename.",
                existing.name
            );
        }
    }

    ensure_structure(&path, name, description)?;

    // Git init if not already a repo
    if !path.join(".git").exists() {
        git::init_repo(&path)?;
    }

    // Initial commit
    let commit_result = git::commit(&path, &format!("create: {name}"));
    if let Ok(ref hash) = commit_result {
        if !hash.is_empty() {
            committed = true;
        }
    }

    // Register
    let entry = WikiEntry {
        name: name.into(),
        path: path.to_string_lossy().into(),
        description: description.map(|s| s.into()),
        remote: None,
    };
    register(entry, force, config_path)?;

    // Ensure global engine directories exist
    if let Some(wiki_dir) = config_path.parent() {
        let logs_dir = wiki_dir.join("logs");
        if !logs_dir.exists() {
            std::fs::create_dir_all(&logs_dir)?;
        }
    }

    if set_default {
        set_default_wiki(name, config_path)?;
    }

    Ok(CreateReport {
        path: path.to_string_lossy().into(),
        name: name.into(),
        created,
        registered: true,
        committed,
    })
}

fn ensure_structure(path: &Path, name: &str, description: Option<&str>) -> Result<()> {
    for dir in &["inbox", "raw", "wiki", "schemas"] {
        let d = path.join(dir);
        if !d.exists() {
            std::fs::create_dir_all(&d)?;
        }
        let gitkeep = d.join(".gitkeep");
        if !gitkeep.exists() {
            std::fs::write(&gitkeep, "")?;
        }
    }

    // Write embedded default schemas
    let schemas_dir = path.join("schemas");
    for (filename, content) in default_schemas() {
        let dest = schemas_dir.join(filename);
        if !dest.exists() {
            std::fs::write(&dest, content)?;
        }
    }

    let readme = path.join("README.md");
    if !readme.exists() {
        let desc_line = description.map(|d| format!("\n{d}\n")).unwrap_or_default();
        let content = format!(
            "# {name}\n{desc_line}\nManaged by [llm-wiki](https://github.com/geronimo-iia/llm-wiki). Run `llm-wiki serve` to start the MCP server.\n"
        );
        std::fs::write(&readme, content)?;
    }

    let wiki_toml = path.join("wiki.toml");
    if !wiki_toml.exists() {
        std::fs::write(&wiki_toml, generate_wiki_toml(name, description))?;
    }

    Ok(())
}

fn generate_wiki_toml(name: &str, description: Option<&str>) -> String {
    let mut s = format!("name = \"{name}\"\n");
    if let Some(desc) = description {
        s.push_str(&format!("description = \"{desc}\"\n"));
    }
    s
}

// ── Space management ──────────────────────────────────────────────────────────

pub fn resolve_name(name: &str, global: &GlobalConfig) -> Result<WikiEntry> {
    global
        .wikis
        .iter()
        .find(|w| w.name == name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("wiki \"{name}\" is not registered"))
}

pub fn register(entry: WikiEntry, force: bool, config_path: &Path) -> Result<()> {
    let mut config = load_global(config_path)?;

    if let Some(existing) = config.wikis.iter_mut().find(|w| w.name == entry.name) {
        if !force {
            bail!(
                "wiki already registered as \"{}\". Use --force to update.",
                entry.name
            );
        }
        *existing = entry;
    } else {
        config.wikis.push(entry);
    }

    save_global(&config, config_path)
}

pub fn remove(name: &str, delete: bool, config_path: &Path) -> Result<()> {
    let mut config = load_global(config_path)?;

    if config.global.default_wiki == name {
        bail!("\"{name}\" is the default wiki \u{2014} set a new default first");
    }

    let idx = config
        .wikis
        .iter()
        .position(|w| w.name == name)
        .ok_or_else(|| anyhow::anyhow!("wiki \"{name}\" is not registered"))?;

    let entry = config.wikis.remove(idx);

    if delete {
        let path = Path::new(&entry.path);
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
    }

    save_global(&config, config_path)
}

pub fn load_all(global: &GlobalConfig) -> Vec<WikiEntry> {
    global.wikis.clone()
}

pub fn set_default_wiki(name: &str, config_path: &Path) -> Result<()> {
    let mut config = load_global(config_path)?;

    if !config.wikis.iter().any(|w| w.name == name) {
        bail!("wiki \"{name}\" is not registered");
    }

    config.global.default_wiki = name.to_string();
    save_global(&config, config_path)
}
