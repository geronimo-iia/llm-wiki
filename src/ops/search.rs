use anyhow::Result;

use crate::engine::EngineState;
use crate::search;

pub struct SearchParams<'a> {
    pub query: &'a str,
    pub type_filter: Option<&'a str>,
    pub no_excerpt: bool,
    pub top_k: Option<usize>,
    pub include_sections: bool,
    pub cross_wiki: bool,
}

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
