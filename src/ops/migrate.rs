use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::config::GlobalConfig;
use crate::default_schemas::is_stock_schema;

#[derive(Debug, Serialize)]
pub struct WikiMigrateReport {
    pub name: String,
    pub deleted: Vec<String>,
    pub kept_custom: Vec<String>,
    pub already_clean: bool,
}

#[derive(Debug, Serialize)]
pub struct MigrateReport {
    pub wikis: Vec<WikiMigrateReport>,
}

pub fn wiki_migrate(
    config: &GlobalConfig,
    wiki_name: Option<&str>,
    dry_run: bool,
) -> Result<MigrateReport> {
    let wikis_to_process: Vec<_> = match wiki_name {
        Some(name) => config.wikis.iter().filter(|w| w.name == name).collect(),
        None => config.wikis.iter().collect(),
    };

    let mut reports = Vec::new();
    for entry in wikis_to_process {
        reports.push(migrate_one(&entry.name, &entry.path, dry_run)?);
    }
    Ok(MigrateReport { wikis: reports })
}

fn migrate_one(name: &str, wiki_path: &Path, dry_run: bool) -> Result<WikiMigrateReport> {
    let schemas_dir = wiki_path.join("schemas");
    let mut deleted = Vec::new();
    let mut kept_custom = Vec::new();

    if schemas_dir.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(&schemas_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let content = std::fs::read_to_string(&path)?;

            if is_stock_schema(&content) {
                if !dry_run {
                    std::fs::remove_file(&path)?;
                }
                deleted.push(filename);
            } else {
                kept_custom.push(filename);
            }
        }
    }

    // A wiki with only custom schemas is clean — nothing for migration to do.
    let already_clean = deleted.is_empty();
    Ok(WikiMigrateReport {
        name: name.to_owned(),
        deleted,
        kept_custom,
        already_clean,
    })
}
