use std::path::Path;

use anyhow::{bail, Result};

use crate::config::{load_global, save_global, GlobalConfig, WikiEntry};

pub fn resolve_uri(uri: &str, global: &GlobalConfig) -> Result<(WikiEntry, String)> {
    let stripped = uri
        .strip_prefix("wiki://")
        .ok_or_else(|| anyhow::anyhow!("invalid wiki URI: {uri}"))?;

    if stripped.is_empty() {
        bail!("invalid wiki URI: {uri}");
    }

    let parts: Vec<&str> = stripped.splitn(2, '/').collect();

    // Try as wiki_name/slug first
    if parts.len() == 2 {
        if let Some(entry) = global.wikis.iter().find(|w| w.name == parts[0]) {
            return Ok((entry.clone(), parts[1].to_string()));
        }
    }

    // Fall back to default wiki with full path as slug
    let default = &global.global.default_wiki;
    if default.is_empty() {
        bail!("no default wiki configured and wiki name not found in URI: {uri}");
    }
    let entry = global
        .wikis
        .iter()
        .find(|w| w.name == *default)
        .ok_or_else(|| anyhow::anyhow!("default wiki \"{default}\" is not registered"))?;
    Ok((entry.clone(), stripped.to_string()))
}

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
        bail!("\"{name}\" is the default wiki — set a new default first");
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

pub fn set_default(name: &str, config_path: &Path) -> Result<()> {
    let mut config = load_global(config_path)?;

    if !config.wikis.iter().any(|w| w.name == name) {
        bail!("wiki \"{name}\" is not registered");
    }

    config.global.default_wiki = name.to_string();
    save_global(&config, config_path)
}
