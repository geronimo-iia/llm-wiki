use anyhow::Result;

use crate::engine::EngineState;
use crate::search;

/// Parameters for the `search` operation.
pub struct SearchParams<'a> {
    /// Full-text query string.
    pub query: &'a str,
    /// Restrict results to this frontmatter type.
    pub type_filter: Option<&'a str>,
    /// When true, omit body excerpts from results.
    pub no_excerpt: bool,
    /// Maximum number of results to return.
    pub top_k: Option<usize>,
    /// When true, include section index pages in results.
    pub include_sections: bool,
    /// When true, search across all mounted wikis.
    pub cross_wiki: bool,
}

/// Run a BM25 search against the wiki index.
pub fn search(
    engine: &EngineState,
    wiki_name: &str,
    params: &SearchParams<'_>,
) -> Result<search::SearchResult> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let opts = search::SearchOptions {
        no_excerpt: params.no_excerpt,
        include_sections: params.include_sections,
        top_k: params
            .top_k
            .unwrap_or(resolved.defaults.search_top_k as usize),
        r#type: params.type_filter.map(|s| s.to_string()),
        facets_top_tags: resolved.defaults.facets_top_tags as usize,
        search_config: resolved.search.clone(),
    };

    if params.cross_wiki {
        let mut wikis = Vec::new();
        for s in engine.spaces.values() {
            let searcher = s.index_manager.searcher()?;
            wikis.push((s.name.clone(), searcher, &s.index_schema));
        }
        return search::search_all(params.query, &opts, &wikis);
    }

    let searcher = space.index_manager.searcher()?;
    search::search(
        params.query,
        &opts,
        &searcher,
        wiki_name,
        &space.index_schema,
    )
}

/// Return a paginated listing of wiki pages with optional type/status filters.
pub fn list(
    engine: &EngineState,
    wiki_name: &str,
    type_filter: Option<&str>,
    status: Option<&str>,
    page: usize,
    page_size: Option<usize>,
) -> Result<search::PageList> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let opts = search::ListOptions {
        r#type: type_filter.map(|s| s.to_string()),
        status: status.map(|s| s.to_string()),
        page,
        page_size: page_size.unwrap_or(resolved.defaults.list_page_size as usize),
        facets_top_tags: resolved.defaults.facets_top_tags as usize,
    };
    let searcher = space.index_manager.searcher()?;
    search::list(&opts, &searcher, wiki_name, &space.index_schema)
}
