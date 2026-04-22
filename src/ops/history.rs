use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::engine::EngineState;
use crate::git;
use crate::slug::{Slug, WikiUri};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResult {
    pub slug: String,
    pub entries: Vec<git::HistoryEntry>,
}

pub fn history(
    engine: &EngineState,
    slug_or_uri: &str,
    wiki_flag: Option<&str>,
    limit: Option<usize>,
    follow: Option<bool>,
) -> Result<HistoryResult> {
    let (wiki_name, slug) = if slug_or_uri.starts_with("wiki://") {
        let (entry, slug) = WikiUri::resolve(slug_or_uri, wiki_flag, &engine.config)?;
        (entry.name, slug)
    } else {
        let wiki_name = engine.resolve_wiki_name(wiki_flag).to_string();
        let slug = Slug::try_from(slug_or_uri)?;
        (wiki_name, slug)
    };

    let space = engine.space(&wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let limit = limit.unwrap_or(resolved.history.default_limit as usize);
    let follow = follow.unwrap_or(resolved.history.follow);

    // Resolve slug to absolute path, then make relative to repo root
    let abs_path = slug.resolve(&space.wiki_root)?;
    let rel_path = abs_path.strip_prefix(&space.repo_root).unwrap_or(&abs_path);

    let entries = git::page_history(&space.repo_root, rel_path, limit, follow)?;

    Ok(HistoryResult {
        slug: slug.to_string(),
        entries,
    })
}
