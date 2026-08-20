#![allow(unreachable_pub)]
use std::cmp::Reverse;
use std::collections::HashMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use tantivy::{
    DocId, Order, Score, Searcher, Term,
    collector::{Collector, Count, MultiCollector, SegmentCollector, TopDocs},
    query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Value},
    snippet::{Snippet, SnippetGenerator},
};

use crate::config::SearchConfig;
use crate::index_schema::IndexSchema;

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

// ── Options ───────────────────────────────────────────────────────────────────

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
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            r#type: None,
            status: None,
            page: 1,
            page_size: 20,
            facets_top_tags: 10,
        }
    }
}

// ── search ────────────────────────────────────────────────────────────────────

/// Run a BM25 full-text search against a single wiki's index.
pub fn search(
    query_str: &str,
    options: &SearchOptions,
    searcher: &Searcher,
    wiki_name: &str,
    is: &IndexSchema,
) -> Result<SearchResult> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_summary = is.try_field("summary");
    let f_body = is.field("body");
    let f_type = is.field("type");

    let index = searcher.index();
    let mut query_fields = vec![f_title, f_body];
    if let Some(f) = f_summary {
        query_fields.insert(1, f);
    }
    let query_parser = QueryParser::for_index(index, query_fields);
    // Lenient fallback: queries containing colons or field specifiers (e.g. "title:foo")
    // are rejected by the strict parser. The lenient parser silently discards invalid
    // tokens and returns the rest of the query rather than failing the search call.
    // Pinned by: src/search.rs tests::colon_query_uses_lenient_fallback.
    let parsed = query_parser
        .parse_query(query_str)
        .unwrap_or_else(|_| query_parser.parse_query_lenient(query_str).0);

    // Build the filtered query (with type filter)
    let final_query: Box<dyn tantivy::query::Query> = {
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
        clauses.push((Occur::Must, parsed));

        if !options.include_sections {
            clauses.push((
                Occur::MustNot,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, "section"),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some(ref type_filter) = options.r#type {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, type_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        Box::new(BooleanQuery::new(clauses))
    };

    let sc = options.search_config.clone();
    let has_confidence = is.try_field("confidence").is_some();
    let collector = TopDocs::with_limit(options.top_k).tweak_score(
        move |segment_reader: &tantivy::SegmentReader| {
            let status_col = segment_reader.fast_fields().str("status").ok().flatten();
            let conf_col = if has_confidence {
                segment_reader.fast_fields().f64("confidence").ok()
            } else {
                None
            };
            let status_map = sc.status.clone();
            move |doc: DocId, score: Score| {
                let unknown_mult = status_map.get("unknown").copied().unwrap_or(0.9);
                let status_mult = match &status_col {
                    Some(col) => match col.term_ords(doc).next() {
                        Some(ord) => {
                            let mut buf = String::new();
                            col.ord_to_str(ord, &mut buf).ok();
                            status_map
                                .get(buf.as_str())
                                .copied()
                                .unwrap_or(unknown_mult)
                        }
                        None => unknown_mult,
                    },
                    None => unknown_mult,
                };
                // Absent confidence is neutral (1.0): pages that don't
                // declare confidence are not down-ranked.
                let confidence = conf_col.as_ref().and_then(|c| c.first(doc)).unwrap_or(1.0) as f32;
                score * status_mult * confidence
            }
        },
    );
    let mut multi = MultiCollector::new();
    let top_docs_handle = multi.add_collector(collector);
    let status_handle = multi.add_collector(KeywordFacetCollector {
        field_name: "status".to_string(),
        top_n: 0,
    });
    let tags_handle = multi.add_collector(KeywordFacetCollector {
        field_name: "tags".to_string(),
        top_n: options.facets_top_tags,
    });
    let mut multi_fruit = searcher.search(&final_query, &multi)?;
    let top_docs = top_docs_handle.extract(&mut multi_fruit);
    let status_facet = status_handle.extract(&mut multi_fruit);
    let tags_facet = tags_handle.extract(&mut multi_fruit);

    let snippet_gen = if !options.no_excerpt {
        Some(SnippetGenerator::create(searcher, &final_query, f_body)?)
    } else {
        None
    };

    let f_confidence = is.try_field("confidence");

    let mut results = Vec::new();
    for (score, doc_addr) in top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(doc_addr)?;

        // Slug stored in the Tantivy index was written via Slug::normalize() at
        // index time, so the value is already lowercase-validated. Skip re-normalization.
        let slug = crate::slug::NormalizedSlug::from_normalized(
            doc.get_first(f_slug)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        );
        let title = doc
            .get_first(f_title)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let uri = format!("wiki://{wiki_name}/{slug}");

        let confidence = f_confidence
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let excerpt = snippet_gen.as_ref().map(|sg| {
            let snippet: Snippet = sg.snippet_from_doc(&doc);
            snippet.to_html()
        });

        let summary = f_summary
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        results.push(PageRef {
            slug,
            uri,
            title,
            score,
            confidence,
            excerpt,
            summary,
        });
    }

    // Facets: type is unfiltered, status and tags are filtered
    // Re-parse query for the unfiltered facet query (same lenient fallback as above).
    let unfiltered_query: Box<dyn tantivy::query::Query> = {
        let parsed2 = query_parser
            .parse_query(query_str)
            .unwrap_or_else(|_| query_parser.parse_query_lenient(query_str).0);
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
        clauses.push((Occur::Must, parsed2));
        if !options.include_sections {
            clauses.push((
                Occur::MustNot,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, "section"),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        Box::new(BooleanQuery::new(clauses))
    };

    let mut type_facets = collect_facets(searcher, &unfiltered_query, &[("type", 0)])?;
    let type_facet = type_facets.remove(0);

    Ok(SearchResult {
        results,
        facets: FacetCounts {
            r#type: type_facet,
            status: status_facet,
            tags: tags_facet,
        },
    })
}

// ── list ──────────────────────────────────────────────────────────────────────

/// Return a paginated list of pages from the index, sorted alphabetically by slug.
pub fn list(
    options: &ListOptions,
    searcher: &Searcher,
    wiki_name: &str,
    is: &IndexSchema,
) -> Result<PageList> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_type = is.field("type");
    let f_status = is.field("status");
    let f_tags = is.field("tags");
    let f_confidence = is.try_field("confidence");
    let f_summary = is.try_field("summary");

    let query: Box<dyn tantivy::query::Query> = {
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        if let Some(ref type_filter) = options.r#type {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, type_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some(ref status_filter) = options.status {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_status, status_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if clauses.is_empty() {
            Box::new(AllQuery)
        } else {
            Box::new(BooleanQuery::new(clauses))
        }
    };

    // Unfiltered query for type facet (no type/status filter)
    let unfiltered_query: Box<dyn tantivy::query::Query> = Box::new(AllQuery);

    let page = options.page;
    let page_size = options.page_size;
    if page_size == 0 {
        bail!("page_size must be at least 1");
    }
    let offset = (page - 1) * page_size;
    let limit = offset + page_size;

    let mut multi = MultiCollector::new();
    let count_handle = multi.add_collector(Count);
    let top_docs_handle = multi
        .add_collector(TopDocs::with_limit(limit).order_by_string_fast_field("slug", Order::Asc));
    let status_handle = multi.add_collector(KeywordFacetCollector {
        field_name: "status".to_string(),
        top_n: 0,
    });
    let tags_handle = multi.add_collector(KeywordFacetCollector {
        field_name: "tags".to_string(),
        top_n: options.facets_top_tags,
    });
    let mut multi_fruit = searcher.search(&query, &multi)?;
    let total = count_handle.extract(&mut multi_fruit);
    let sorted_docs = top_docs_handle.extract(&mut multi_fruit);
    let status_facet = status_handle.extract(&mut multi_fruit);
    let tags_facet = tags_handle.extract(&mut multi_fruit);

    if total == 0 {
        // Still collect facets even with no results in the page window
        let mut type_facets = collect_facets(searcher, &unfiltered_query, &[("type", 0)])?;
        return Ok(PageList {
            pages: Vec::new(),
            total: 0,
            page,
            page_size,
            facets: FacetCounts {
                r#type: type_facets.remove(0),
                status: status_facet,
                tags: tags_facet,
            },
        });
    }

    // Extract full fields only for the page window
    let window = if offset < sorted_docs.len() {
        &sorted_docs[offset..]
    } else {
        &[]
    };

    let mut summaries = Vec::with_capacity(window.len());
    for (_slug_val, doc_addr) in window {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_addr)?;

        // See comment above: Tantivy index stores pre-normalized slugs.
        let slug = crate::slug::NormalizedSlug::from_normalized(
            doc.get_first(f_slug)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        );
        let title = doc
            .get_first(f_title)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let status = doc
            .get_first(f_status)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tags: Vec<String> = doc
            .get_all(f_tags)
            .filter_map(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let confidence = f_confidence
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let summary = f_summary
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let uri = format!("wiki://{wiki_name}/{slug}");

        summaries.push(PageSummary {
            slug,
            uri,
            title,
            r#type: page_type,
            status,
            tags,
            confidence,
            summary,
        });
    }

    Ok(PageList {
        pages: summaries,
        total,
        page,
        page_size,
        facets: {
            let mut type_facets = collect_facets(searcher, &unfiltered_query, &[("type", 0)])?;
            FacetCounts {
                r#type: type_facets.remove(0),
                status: status_facet,
                tags: tags_facet,
            }
        },
    })
}

// ── search_all ────────────────────────────────────────────────────────────────

/// Search across multiple wikis, merge results by score, and truncate to `top_k`.
pub fn search_all(
    query_str: &str,
    options: &SearchOptions,
    wikis: &[(String, Searcher, &IndexSchema)],
) -> Result<SearchResult> {
    let mut all_results = Vec::new();
    let mut merged_facets = FacetCounts::default();
    for (name, searcher, is) in wikis {
        match search(query_str, options, searcher, name, is) {
            Ok(sr) => {
                all_results.extend(sr.results);
                for (k, v) in sr.facets.r#type {
                    *merged_facets.r#type.entry(k).or_insert(0) += v;
                }
                for (k, v) in sr.facets.status {
                    *merged_facets.status.entry(k).or_insert(0) += v;
                }
                for (k, v) in sr.facets.tags {
                    *merged_facets.tags.entry(k).or_insert(0) += v;
                }
            }
            Err(e) => {
                tracing::warn!(wiki = %name, error = %e, "cross-wiki search failed for wiki; skipping");
                continue;
            }
        }
    }
    all_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.top_k);

    // Re-cap tags after merging
    if options.facets_top_tags > 0 && merged_facets.tags.len() > options.facets_top_tags {
        let mut entries: Vec<_> = merged_facets.tags.into_iter().collect();
        entries.sort_by_key(|e| Reverse(e.1));
        entries.truncate(options.facets_top_tags);
        merged_facets.tags = entries.into_iter().collect();
    }

    Ok(SearchResult {
        results: all_results,
        facets: merged_facets,
    })
}

// ── Facet collection ──────────────────────────────────────────────────────────

struct KeywordFacetCollector {
    field_name: String,
    top_n: usize,
}

struct KeywordFacetSegmentCollector {
    column: Option<tantivy::columnar::StrColumn>,
    buf: String,
    counts: HashMap<String, u64>,
}

impl Collector for KeywordFacetCollector {
    type Fruit = HashMap<String, u64>;
    type Child = KeywordFacetSegmentCollector;

    fn for_segment(
        &self,
        _segment_local_id: u32,
        reader: &tantivy::SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        let column = reader.fast_fields().str(&self.field_name).ok().flatten();
        Ok(KeywordFacetSegmentCollector {
            column,
            buf: String::new(),
            counts: HashMap::new(),
        })
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(
        &self,
        fruits: Vec<HashMap<String, u64>>,
    ) -> tantivy::Result<HashMap<String, u64>> {
        let mut merged: HashMap<String, u64> = HashMap::new();
        for f in fruits {
            for (k, v) in f {
                *merged.entry(k).or_insert(0) += v;
            }
        }
        if self.top_n > 0 && merged.len() > self.top_n {
            let mut entries: Vec<_> = merged.into_iter().collect();
            entries.sort_by_key(|e| Reverse(e.1));
            entries.truncate(self.top_n);
            return Ok(entries.into_iter().collect());
        }
        Ok(merged)
    }
}

impl SegmentCollector for KeywordFacetSegmentCollector {
    type Fruit = HashMap<String, u64>;

    fn collect(&mut self, doc: tantivy::DocId, _score: tantivy::Score) {
        let Some(col) = &self.column else { return };
        for ord in col.term_ords(doc) {
            self.buf.clear();
            if col.ord_to_str(ord, &mut self.buf).unwrap_or(false) && !self.buf.is_empty() {
                *self.counts.entry(self.buf.clone()).or_insert(0) += 1;
            }
        }
    }

    fn harvest(self) -> HashMap<String, u64> {
        self.counts
    }
}

fn collect_facets(
    searcher: &Searcher,
    query: &dyn tantivy::query::Query,
    fields: &[(&str, usize)],
) -> Result<Vec<HashMap<String, u64>>> {
    if fields.is_empty() {
        return Ok(Vec::new());
    }
    let mut multi = MultiCollector::new();
    let handles: Vec<_> = fields
        .iter()
        .map(|(name, top_n)| {
            multi.add_collector(KeywordFacetCollector {
                field_name: name.to_string(),
                top_n: *top_n,
            })
        })
        .collect();
    let mut fruits = searcher.search(query, &multi)?;
    Ok(handles
        .into_iter()
        .map(|h| h.extract(&mut fruits))
        .collect())
}

// ── llms renderers ────────────────────────────────────────────────────────────

/// Render a `PageList` as LLM-optimized markdown: pages grouped by type,
/// one line per page with summary. Archived pages shown with strikethrough.
pub fn render_list_llms(result: &PageList) -> String {
    // Group by type, sorted by count desc then name asc
    let mut by_type: std::collections::HashMap<String, Vec<&PageSummary>> =
        std::collections::HashMap::new();
    for page in &result.pages {
        by_type.entry(page.r#type.clone()).or_default().push(page);
    }
    let mut groups: Vec<(String, Vec<&PageSummary>)> = by_type.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));

    let mut out = String::new();
    for (type_name, mut pages) in groups {
        pages.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.title.cmp(&b.title))
        });
        out.push_str(&format!("## {} ({})\n\n", type_name, pages.len()));
        for page in pages {
            let summary = page.summary.as_deref().unwrap_or("");
            let line = if page.status == "archived" {
                if summary.is_empty() {
                    format!("- ~~[{}]({})~~\n", page.title, page.uri)
                } else {
                    format!("- ~~[{}]({}): {}~~\n", page.title, page.uri, summary)
                }
            } else if summary.is_empty() {
                format!("- [{}]({})\n", page.title, page.uri)
            } else {
                format!("- [{}]({}): {}\n", page.title, page.uri, summary)
            };
            out.push_str(&line);
        }
        out.push('\n');
    }

    if result.total > result.page_size {
        let total_pages = (result.total + result.page_size - 1) / result.page_size.max(1);
        out.push_str(&format!(
            "_Page {}/{} — {} total pages_\n",
            result.page, total_pages, result.total
        ));
    }

    out
}

/// Render a `SearchResult` as LLM-optimized markdown: one line per result
/// with title, uri, and summary. No score, no excerpt block.
pub fn render_search_llms(result: &SearchResult) -> String {
    if result.results.is_empty() {
        return "No results found.\n".to_string();
    }
    let mut out = String::new();
    for r in &result.results {
        let summary = r.summary.as_deref().unwrap_or("");
        if summary.is_empty() {
            out.push_str(&format!("- [{}]({})\n", r.title, r.uri));
        } else {
            out.push_str(&format!("- [{}]({}): {}\n", r.title, r.uri, summary));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use tantivy::Index;
    use tantivy::query::QueryParser;
    use tantivy::schema::{SchemaBuilder, TEXT};

    /// `parse_query` fails on bare field specifiers like `title:attention` when
    /// `title` is not a registered query field. `parse_query_lenient` must
    /// succeed and return a usable query rather than propagating the error.
    #[test]
    fn parse_query_lenient_fallback_on_field_specifier() {
        let mut builder = SchemaBuilder::new();
        let body = builder.add_text_field("body", TEXT);
        let schema = builder.build();
        let index = Index::create_in_ram(schema);
        let parser = QueryParser::for_index(&index, vec![body]);

        // `title:attention` fails strict parse (title not in query fields)
        assert!(parser.parse_query("title:attention").is_err());
        // lenient parse must succeed
        let (query, _errors) = parser.parse_query_lenient("title:attention");
        // the returned query must be usable (searcher.search won't panic)
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let count = searcher.search(&query, &tantivy::collector::Count).unwrap();
        assert_eq!(count, 0); // empty index — just verifying no panic
    }

    /// Type-filter query: indexing two docs with different types and filtering on one
    /// must return only the matching doc. Mirrors the BooleanQuery + TermQuery path
    /// in the production `search()` function.
    #[test]
    fn type_filter_excludes_non_matching_type() {
        use tantivy::doc;
        use tantivy::query::{BooleanQuery, Occur, TermQuery};
        use tantivy::schema::{IndexRecordOption, STRING, SchemaBuilder, TEXT};
        use tantivy::{Index, Term};

        let mut builder = SchemaBuilder::new();
        let f_body = builder.add_text_field("body", TEXT);
        let f_type = builder.add_text_field("type", STRING);
        let schema = builder.build();
        let index = Index::create_in_ram(schema.clone());

        let mut writer = index.writer(15_000_000).unwrap();
        writer
            .add_document(doc!(f_body => "attention mechanism", f_type => "concept"))
            .unwrap();
        writer
            .add_document(doc!(f_body => "mixtral paper", f_type => "source"))
            .unwrap();
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let parser = QueryParser::for_index(&index, vec![f_body]);
        let base = parser.parse_query("attention").unwrap();

        // Filter: type must be "concept"
        let type_term = Term::from_field_text(f_type, "concept");
        let filtered = BooleanQuery::new(vec![
            (Occur::Must, base),
            (
                Occur::Must,
                Box::new(TermQuery::new(type_term, IndexRecordOption::Basic)),
            ),
        ]);

        let count = searcher
            .search(&filtered, &tantivy::collector::Count)
            .unwrap();
        assert_eq!(count, 1, "type filter must return only the concept doc");
    }
}
