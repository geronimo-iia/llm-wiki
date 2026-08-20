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
    let do_create = || {
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
            && let Some(e) = engine
        {
            let entry = WikiEntry {
                name: name.to_string(),
                path: std::path::PathBuf::from(&report.path),
                description: description.map(|s| s.to_string()),
                remote: None,
            };
            // Roll back config entry if mount fails — inside the lock so no
            // concurrent config mutation can interleave with the rollback.
            if let Err(mount_err) = e.mount_wiki(&entry) {
                let _ = spaces::remove(name, false, config_path).inspect_err(
                    |e| tracing::error!(error = %e, "rollback failed; wiki may be stranded in config"),
                );
                return Err(mount_err);
            }
        }
        Ok(report)
    };

    let report = match engine {
        Some(e) => e.with_config_lock(do_create)?,
        None => do_create()?,
    };

    // set_default is outside with_config_lock. Two concurrent spaces_create calls with
    // set_default: true will both succeed; the last set_default wins non-deterministically.
    // No data is corrupted — the result is a valid registered wiki as default. This is
    // last-writer-wins by design. Moving set_default inside the closure is tracked in D5-1.
    if report.registered
        && set_default
        && let Some(e) = engine
    {
        e.set_default(name)?;
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
    let do_register = || {
        let report = spaces::register_existing(path, name, description, wiki_root, config_path)?;
        if report.registered
            && let Some(e) = engine
        {
            let entry = WikiEntry {
                name: name.to_string(),
                path: std::path::PathBuf::from(&report.path),
                description: description.map(|s| s.to_string()),
                remote: None,
            };
            if let Err(mount_err) = e.mount_wiki(&entry) {
                let _ = spaces::remove(name, false, config_path).inspect_err(
                    |e| tracing::error!(error = %e, "rollback failed; wiki may be stranded in config"),
                );
                return Err(mount_err);
            }
        }
        Ok(report)
    };
    match engine {
        Some(e) => e.with_config_lock(do_register),
        None => do_register(),
    }
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
    let do_remove = || {
        // Hot reload: unmount before removing from config
        if let Some(engine) = engine {
            engine.unmount_wiki(name)?;
        }
        spaces::remove(name, delete, config_path)
    };
    match engine {
        Some(e) => e.with_config_lock(do_remove),
        None => do_remove(),
    }
}

/// Set the default wiki in config and update the running engine.
pub fn spaces_set_default(
    name: &str,
    config_path: &Path,
    engine: Option<&WikiEngine>,
) -> Result<()> {
    let do_set_default = || {
        if let Some(engine) = engine {
            // Capture previous in-memory default for rollback.
            let prev = engine
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("lock poisoned"))?
                .config
                .global
                .default_wiki
                .clone();

            // Validate (wiki not mounted → error before touching disk) and update in-memory.
            engine.set_default(name)?;

            // Persist to disk. On failure, restore the previous in-memory value.
            if let Err(disk_err) = spaces::set_default_wiki(name, config_path) {
                let mut eng = engine.state.write().unwrap_or_else(|e| e.into_inner());
                eng.config.global.default_wiki = prev;
                return Err(disk_err);
            }
        } else {
            spaces::set_default_wiki(name, config_path)?;
        }
        Ok(())
    };
    match engine {
        Some(e) => e.with_config_lock(do_set_default),
        None => do_set_default(),
    }
}
