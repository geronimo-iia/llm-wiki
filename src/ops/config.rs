use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::{self, WikiConfig};
use crate::spaces;

/// Read a single config key from the resolved global config.
pub fn config_get(config_path: &Path, key: &str) -> Result<String> {
    let g = config::load_global(config_path)?;
    let resolved = config::resolve(&g, &WikiConfig::default());
    Ok(config::get_config_value(&resolved, &g, key))
}

/// Set a config key in either the global or wiki-level config file.
pub fn config_set(
    config_path: &Path,
    key: &str,
    value: &str,
    global: bool,
    wiki_name: Option<&str>,
) -> Result<String> {
    if global {
        let mut g = config::load_global(config_path)?;
        config::set_global_config_value(&mut g, key, value)?;
        config::save_global(&g, config_path)?;
        Ok(format!("Set {key} = {value} (global)"))
    } else {
        let g = config::load_global(config_path)?;
        let name = wiki_name.unwrap_or(&g.global.default_wiki);
        let entry = spaces::resolve_name(name, &g)?;
        let entry_path = PathBuf::from(&entry.path);
        let mut wiki_cfg = config::load_wiki(&entry_path)?;
        config::set_wiki_config_value(&mut wiki_cfg, key, value)?;
        config::save_wiki(&wiki_cfg, &entry_path)?;
        Ok(format!("Set {key} = {value} (wiki: {name})"))
    }
}

/// Serialize the global config to a TOML string.
pub fn config_list_global(config_path: &Path) -> Result<String> {
    let g = config::load_global(config_path)?;
    Ok(toml::to_string_pretty(&g)?)
}

/// Return the fully resolved config (global defaults merged with wiki overrides).
pub fn config_list_resolved(config_path: &Path) -> Result<config::ResolvedConfig> {
    let g = config::load_global(config_path)?;
    Ok(config::resolve(&g, &WikiConfig::default()))
}
