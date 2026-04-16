use std::path::Path;

use anyhow::{bail, Result};

use crate::config::{load_global, WikiEntry};
use crate::git;
use crate::spaces;

#[derive(Debug)]
pub struct InitReport {
    pub path: String,
    pub name: String,
    pub created: bool,
    pub registered: bool,
    pub committed: bool,
}

pub fn init(
    path: &Path,
    name: &str,
    description: Option<&str>,
    force: bool,
    set_default: bool,
    config_path: &Path,
) -> Result<InitReport> {
    // Create directory first so canonicalize always resolves symlinks
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
            return Ok(InitReport {
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

    // Ensure structure (directory already created above)
    ensure_structure(&path, name, description)?;

    // Git init if not already a repo
    if !path.join(".git").exists() {
        git::init_repo(&path)?;
    }

    // Initial commit
    let commit_result = git::commit(&path, &format!("init: {name}"));
    if commit_result.is_ok() {
        committed = true;
    }

    // Register
    let entry = WikiEntry {
        name: name.into(),
        path: path.to_string_lossy().into(),
        description: description.map(|s| s.into()),
        remote: None,
    };
    spaces::register(entry, force, config_path)?;

    // Ensure global engine directories exist
    if let Some(wiki_dir) = config_path.parent() {
        let logs_dir = wiki_dir.join("logs");
        if !logs_dir.exists() {
            std::fs::create_dir_all(&logs_dir)?;
        }
    }

    if set_default {
        spaces::set_default(name, config_path)?;
    }

    Ok(InitReport {
        path: path.to_string_lossy().into(),
        name: name.into(),
        created,
        registered: true,
        committed,
    })
}

fn ensure_structure(path: &Path, name: &str, description: Option<&str>) -> Result<()> {
    for dir in &["inbox", "raw", "wiki"] {
        let d = path.join(dir);
        if !d.exists() {
            std::fs::create_dir_all(&d)?;
        }
        let gitkeep = d.join(".gitkeep");
        if !gitkeep.exists() {
            std::fs::write(&gitkeep, "")?;
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
        let mut content = format!("name = \"{name}\"\n");
        if let Some(desc) = description {
            content.push_str(&format!("description = \"{desc}\"\n"));
        }
        std::fs::write(&wiki_toml, content)?;
    }

    let schema = path.join("schema.md");
    if !schema.exists() {
        let content = "# Schema\n\nWiki conventions for this knowledge base.\n\n## Categories\n\n- `concepts/` — concept pages\n- `sources/` — source material\n- `queries/` — query results\n";
        std::fs::write(&schema, content)?;
    }

    Ok(())
}
