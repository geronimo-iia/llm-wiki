use std::path::Path;

use anyhow::Result;

use crate::config::{self, GlobalConfig};
use crate::spaces;

pub fn spaces_create(
    path: &Path,
    name: &str,
    description: Option<&str>,
    force: bool,
    set_default: bool,
    config_path: &Path,
) -> Result<spaces::CreateReport> {
    spaces::create(path, name, description, force, set_default, config_path)
}

pub fn spaces_list(config: &GlobalConfig, name: Option<&str>) -> Vec<config::WikiEntry> {
    let all = spaces::load_all(config);
    match name {
        Some(n) => all.into_iter().filter(|e| e.name == n).collect(),
        None => all,
    }
}

pub fn spaces_remove(name: &str, delete: bool, config_path: &Path) -> Result<()> {
    spaces::remove(name, delete, config_path)
}

pub fn spaces_set_default(name: &str, config_path: &Path) -> Result<()> {
    spaces::set_default_wiki(name, config_path)
}
