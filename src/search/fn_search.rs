use anyhow::Result;
use tantivy::{
    DocId, Score, Searcher, Term,
    collector::{MultiCollector, TopDocs},
    query::{BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Value},
    snippet::{Snippet, SnippetGenerator},
};

use crate::index_schema::IndexSchema;

use super::facets::{KeywordFacetCollector, collect_facets};
use super::options::{SearchOptions, TagsMode};
use super::types::{FacetCounts, PageRef, SearchResult};

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

    let f_status = is.field("status");
    let f_tags = is.field("tags");

    // Build the filtered query (with type, status, tags filters)
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

        if let Some(ref status_filter) = options.status {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_status, status_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if !options.tags.is_empty() {
            match options.tags_mode {
                TagsMode::And => {
                    for tag in &options.tags {
                        clauses.push((
                            Occur::Must,
                            Box::new(TermQuery::new(
                                Term::from_field_text(f_tags, tag),
                                IndexRecordOption::Basic,
                            )),
                        ));
                    }
                }
                TagsMode::Or => {
                    let or_clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = options
                        .tags
                        .iter()
                        .map(|tag| {
                            let q: Box<dyn tantivy::query::Query> = Box::new(TermQuery::new(
                                Term::from_field_text(f_tags, tag),
                                IndexRecordOption::Basic,
                            ));
                            (Occur::Should, q)
                        })
                        .collect();
                    clauses.push((Occur::Must, Box::new(BooleanQuery::new(or_clauses))));
                }
            }
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

    if let Some(min_conf) = options.min_confidence {
        results.retain(|r| r.confidence as f64 >= min_conf);
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
