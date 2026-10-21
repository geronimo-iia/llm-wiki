use anyhow::{Result, bail};
use tantivy::{
    Order, Searcher, Term,
    collector::{Count, MultiCollector, TopDocs},
    query::{AllQuery, BooleanQuery, Occur, TermQuery},
    schema::{IndexRecordOption, Value},
};

use crate::index_schema::IndexSchema;

use super::facets::{KeywordFacetCollector, collect_facets};
use super::options::{ListOptions, SortField, SortOrder, TagsMode};
use super::types::{FacetCounts, PageList, PageSummary};

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

    let tantivy_order = match options.order {
        SortOrder::Asc => Order::Asc,
        SortOrder::Desc => Order::Desc,
    };

    // FruitHandle is monomorphic: order_by_string_fast_field and order_by_fast_field::<f64>
    // return incompatible handle types that cannot share a single match arm. This local enum
    // wraps both variants so we can add exactly one collector to MultiCollector, execute one
    // search, then extract Vec<DocAddress> by matching on the variant.
    use tantivy::collector::FruitHandle;
    enum SortHandle {
        Str(FruitHandle<Vec<(Option<String>, tantivy::DocAddress)>>),
        F64(FruitHandle<Vec<(Option<f64>, tantivy::DocAddress)>>),
    }

    let mut multi = MultiCollector::new();
    let count_handle = multi.add_collector(Count);
    let sort_handle = match options.sort {
        SortField::Slug => SortHandle::Str(multi.add_collector(
            TopDocs::with_limit(limit).order_by_string_fast_field("slug", tantivy_order),
        )),
        SortField::Status => SortHandle::Str(multi.add_collector(
            TopDocs::with_limit(limit).order_by_string_fast_field("status", tantivy_order),
        )),
        SortField::Confidence => SortHandle::F64(multi.add_collector(
            TopDocs::with_limit(limit).order_by_fast_field::<f64>("confidence", tantivy_order),
        )),
    };
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
    let sorted_docs: Vec<tantivy::DocAddress> = match sort_handle {
        SortHandle::Str(h) => h
            .extract(&mut multi_fruit)
            .into_iter()
            .map(|(_, a)| a)
            .collect(),
        SortHandle::F64(h) => h
            .extract(&mut multi_fruit)
            .into_iter()
            .map(|(_, a)| a)
            .collect(),
    };
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
    for doc_addr in window {
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

    if let Some(min_conf) = options.min_confidence {
        summaries.retain(|p| p.confidence as f64 >= min_conf);
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
