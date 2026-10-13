use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Return types ──────────────────────────────────────────────────────────────

/// A single search result with BM25 score and optional highlighted excerpt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRef {
    /// Page slug, normalized (lowercased).
    pub slug: crate::slug::NormalizedSlug,
    /// Fully-qualified `wiki://` URI for the page.
    pub uri: String,
    /// Page title from frontmatter.
    pub title: String,
    /// Adjusted BM25 score (multiplied by status and confidence).
    pub score: f32,
    /// Frontmatter `confidence` value in [0, 1]; 1.0 (neutral) when the
    /// page does not declare one.
    pub confidence: f32,
    /// HTML-highlighted body excerpt, if requested.
    pub excerpt: Option<String>,
    /// Frontmatter `summary` field, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Lightweight page metadata returned by listing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    /// Page slug, normalized (lowercased).
    pub slug: crate::slug::NormalizedSlug,
    /// Fully-qualified `wiki://` URI.
    pub uri: String,
    /// Page title from frontmatter.
    pub title: String,
    /// Page type from frontmatter.
    pub r#type: String,
    /// Page status from frontmatter.
    pub status: String,
    /// Tags from frontmatter.
    pub tags: Vec<String>,
    /// Frontmatter `confidence` value in [0, 1]; 1.0 (neutral) when the
    /// page does not declare one.
    pub confidence: f32,
    /// Frontmatter `summary` field, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// A paginated list of pages with facet counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageList {
    /// Pages in the current page window.
    pub pages: Vec<PageSummary>,
    /// Total pages matching the filter (across all pages).
    pub total: usize,
    /// Current 1-based page number.
    pub page: usize,
    /// Number of items per page.
    pub page_size: usize,
    /// Facet counts for type, status, and tags.
    #[serde(default, skip_serializing_if = "FacetCounts::is_empty")]
    pub facets: FacetCounts,
}

// ── Facets ────────────────────────────────────────────────────────────────────

/// Distribution counts for type, status, and tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FacetCounts {
    /// Count of pages per frontmatter type.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub r#type: HashMap<String, u64>,
    /// Count of pages per frontmatter status.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub status: HashMap<String, u64>,
    /// Count of pages per tag.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, u64>,
}

impl FacetCounts {
    /// Return true if all three facet maps are empty.
    pub fn is_empty(&self) -> bool {
        self.r#type.is_empty() && self.status.is_empty() && self.tags.is_empty()
    }
}

/// The full result of a search query including ranked results and facets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Ranked search results.
    pub results: Vec<PageRef>,
    /// Facet counts for the result set.
    pub facets: FacetCounts,
}
