use std::path::Path;

use anyhow::Result;

use crate::config::{self, GlobalConfig, WikiEntry};
use crate::engine::WikiEngine;
use crate::spaces;

pub fn spaces_create(
    path: &Path,
    name: &str,
    description: Option<&str>,
    force: bool,
    set_default: bool,
    config_path: &Path,
    engine: Option<&WikiEngine>,
) -> Result<spaces::CreateReport> {
    let report = spaces::create(path, name, description, force, set_default, config_path)?;

    // Hot reload: mount the new wiki in the running engine
    if report.registered {
        if let Some(engine) = engine {
            let entry = WikiEntry {
                name: name.to_string(),
                path: report.path.clone(),
                description: description.map(|s| s.to_string()),
                remote: None,
            };
            engine.mount_wiki(&entry)?;
        }
    }

    Ok(report)
}

pub fn spaces_list(config: &GlobalConfig, name: Option<&str>) -> Vec<config::WikiEntry> {
    let all = spaces::load_all(config);
    match name {
        Some(n) => all.into_iter().filter(|e| e.name == n).collect(),
        None => all,
    }
}

pub fn spaces_remove(
    name: &str,
    delete: bool,
    config_path: &Path,
    engine: Option<&WikiEngine>,
) -> Result<()> {
    // Hot reload: unmount before removing from config
    if let Some(engine) = engine {
        engine.unmount_wiki(name)?;
    }
    spaces::remove(name, delete, config_path)
}

pub fn spaces_set_default(
    name: &str,
    config_path: &Path,
    engine: Option<&WikiEngine>,
) -> Result<()> {
    spaces::set_default_wiki(name, config_path)?;

    // Hot reload: update default in the running engine
    if let Some(engine) = engine {
        engine.set_default(name)?;
    }
    Ok(())
}
