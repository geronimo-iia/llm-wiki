use crate::config::SearchConfig;

// ── Options ───────────────────────────────────────────────────────────────────

/// Tag filter match mode.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum TagsMode {
    #[default]
    And,
    Or,
}

/// Sort field for list operations.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    #[default]
    Slug,
    Confidence,
    Status,
}

/// Sort direction.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Options for a BM25 search query.
pub struct SearchOptions {
    /// Omit HTML excerpt from results when true.
    pub no_excerpt: bool,
    /// Include section index pages in results when true.
    pub include_sections: bool,
    /// Maximum number of results to return.
    pub top_k: usize,
    /// Optional frontmatter type filter.
    pub r#type: Option<String>,
    /// Maximum tag facet values to return (0 = all).
    pub facets_top_tags: usize,
    /// Status score multiplier config applied to BM25 scores.
    pub search_config: SearchConfig,
    /// Optional frontmatter status filter.
    pub status: Option<String>,
    /// Tag filter list; empty = no filter.
    pub tags: Vec<String>,
    /// Tag match mode (AND = all must match, OR = any must match).
    pub tags_mode: TagsMode,
    /// Minimum confidence threshold; pages without confidence field always pass.
    pub min_confidence: Option<f64>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            no_excerpt: false,
            include_sections: false,
            top_k: 10,
            r#type: None,
            facets_top_tags: 10,
            search_config: SearchConfig::default(),
            status: None,
            tags: vec![],
            tags_mode: TagsMode::default(),
            min_confidence: None,
        }
    }
}

/// Options for a paginated page list operation.
pub struct ListOptions {
    /// Optional frontmatter type filter.
    pub r#type: Option<String>,
    /// Optional frontmatter status filter.
    pub status: Option<String>,
    /// 1-based page number.
    pub page: usize,
    /// Number of items per page.
    pub page_size: usize,
    /// Maximum tag facet values to return (0 = all).
    pub facets_top_tags: usize,
    /// Tag filter list; empty = no filter.
    pub tags: Vec<String>,
    /// Tag match mode (AND = all must match, OR = any must match).
    pub tags_mode: TagsMode,
    /// Minimum confidence threshold; pages without confidence field always pass.
    pub min_confidence: Option<f64>,
    /// Sort field for list results.
    pub sort: SortField,
    /// Sort direction.
    pub order: SortOrder,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            r#type: None,
            status: None,
            page: 1,
            page_size: 20,
            facets_top_tags: 10,
            tags: vec![],
            tags_mode: TagsMode::default(),
            min_confidence: None,
            sort: SortField::default(),
            order: SortOrder::default(),
        }
    }
}
