use std::path::Path;

use anyhow::Result;

use crate::config::{self, GlobalConfig, WikiEntry};
use crate::engine::WikiEngine;
use crate::spaces;

/// Create a wiki space and hot-reload it into the running engine.
#[allow(clippy::too_many_arguments)]
pub fn spaces_create(
    path: &Path,
    name: &str,
    description: Option<&str>,
    force: bool,
    set_default: bool,
    config_path: &Path,
    engine: Option<&WikiEngine>,
    wiki_root: Option<&str>,
) -> Result<spaces::CreateReport> {
    let report = spaces::create(
        path,
        name,
        description,
        force,
        set_default,
        config_path,
        wiki_root,
    )?;

    if report.registered
        && let Some(engine) = engine
    {
        let entry = WikiEntry {
            name: name.to_string(),
            path: std::path::PathBuf::from(&report.path),
            description: description.map(|s| s.to_string()),
            remote: None,
        };
        // Roll back config entry if mount fails so caller is never left with a
        // registered-but-unmountable wiki.
        if let Err(e) = engine.mount_wiki(&entry) {
            let _ = spaces::remove(name, false, config_path).inspect_err(
                |e| tracing::error!(error = %e, "rollback failed; wiki may be stranded in config"),
            );
            return Err(e);
        }
        if set_default {
            engine.set_default(name)?;
        }
    }

    Ok(report)
}

/// Register an existing wiki space and hot-reload it into the running engine.
pub fn spaces_register(
    path: &Path,
    name: &str,
    description: Option<&str>,
    wiki_root: Option<&str>,
    config_path: &Path,
    engine: Option<&WikiEngine>,
) -> Result<spaces::RegisterReport> {
    let report = spaces::register_existing(path, name, description, wiki_root, config_path)?;

    if report.registered
        && let Some(engine) = engine
    {
        let entry = WikiEntry {
            name: name.to_string(),
            path: std::path::PathBuf::from(&report.path),
            description: description.map(|s| s.to_string()),
            remote: None,
        };
        if let Err(e) = engine.mount_wiki(&entry) {
            let _ = spaces::remove(name, false, config_path).inspect_err(
                |e| tracing::error!(error = %e, "rollback failed; wiki may be stranded in config"),
            );
            return Err(e);
        }
    }

    Ok(report)
}

/// List registered wiki spaces, optionally filtered to a single name.
pub fn spaces_list(config: &GlobalConfig, name: Option<&str>) -> Vec<config::WikiEntry> {
    let all = spaces::load_all(config);
    match name {
        Some(n) => all.into_iter().filter(|e| e.name == n).collect(),
        None => all,
    }
}

/// Unmount a wiki from the engine and remove it from config.
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

/// Set the default wiki in config and update the running engine.
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
