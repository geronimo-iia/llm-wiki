use std::collections::{HashMap, HashSet};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tantivy::schema::Value;

use crate::engine::EngineState;
use crate::graph::{GraphFilter, get_cached_community_map, get_or_build_graph};
use crate::search;
use crate::slug::{Slug, WikiUri};

/// A page suggested as a related link for a given slug.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Slug of the suggested page.
    pub slug: String,
    /// `wiki://` URI of the suggested page.
    pub uri: String,
    /// Display title of the suggested page.
    pub title: String,
    /// Frontmatter type of the suggested page.
    pub r#type: String,
    /// Relevance score (higher is more relevant).
    pub score: f32,
    /// Human-readable reason for the suggestion.
    pub reason: String,
    /// Index field that triggered the suggestion.
    pub field: String,
}

/// Return a ranked list of related-page suggestions for a given slug or URI.
pub fn suggest(
    engine: &EngineState,
    slug_or_uri: &str,
    wiki_flag: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<Suggestion>> {
    let (wiki_name, slug) = if slug_or_uri.starts_with("wiki://") {
        let (entry, slug) = WikiUri::resolve(slug_or_uri, wiki_flag, &engine.config)?;
        (entry.name, slug)
    } else {
        let wiki_name = engine.resolve_wiki_name(wiki_flag)?.to_string();
        let slug = Slug::try_from(slug_or_uri)?;
        (wiki_name, slug)
    };

    let space = engine.space(&wiki_name)?;
    let resolved = space.resolved_config(&engine.config);
    let limit = limit.unwrap_or(resolved.suggest.default_limit as usize);
    let min_score = resolved.suggest.min_score;

    let searcher = space.index_manager.searcher()?;
    let is = &space.index_schema;

    // Read the input page to get its tags, type, and existing links
    let input_doc = find_doc_by_slug(&searcher, is, slug.as_str())?;
    let input_tags: HashSet<String> = input_doc.tags.iter().cloned().collect();
    let input_type = input_doc.page_type.clone();
    let existing_links: HashSet<String> = input_doc.links.iter().cloned().collect();

    let mut candidates: HashMap<String, CandidateScore> = HashMap::new();

    // Strategy 1: Tag overlap
    for tag in &input_tags {
        let results = search::search(
            tag,
            &search::SearchOptions {
                no_excerpt: true,
                top_k: 20,
                ..Default::default()
            },
            &searcher,
            &wiki_name,
            is,
        )?;
        let result_slugs: Vec<&str> = results
            .results
            .iter()
            .filter(|r| r.slug != slug.as_str() && !existing_links.contains(r.slug.as_str()))
            .map(|r| r.slug.as_str())
            .collect();
        let docs = bulk_fetch_docs(&searcher, is, &result_slugs)?;
        for r in &results.results {
            if r.slug == slug.as_str() || existing_links.contains(r.slug.as_str()) {
                continue;
            }
            let doc = docs.get(r.slug.as_str()).cloned().unwrap_or_default();
            let shared: usize = doc.tags.iter().filter(|t| input_tags.contains(*t)).count();
            if shared == 0 {
                continue;
            }
            let total = doc.tags.len().max(1);
            let score = shared as f32 / total as f32;
            let shared_tags: Vec<&str> = doc
                .tags
                .iter()
                .filter(|t| input_tags.contains(*t))
                .map(|s| s.as_str())
                .collect();
            let reason = format!("shares tags: {}", shared_tags.join(", "));
            candidates
                .entry(r.slug.to_string())
                .and_modify(|c| {
                    if score > c.score {
                        c.score = score;
                        c.reason = reason.clone();
                    }
                })
                .or_insert(CandidateScore {
                    slug: r.slug.to_string(),
                    title: r.title.clone(),
                    page_type: doc.page_type.clone(),
                    score,
                    reason,
                });
        }
    }

    // Strategy 2: Graph neighborhood (2 hops)
    let wiki_graph = get_or_build_graph(
        is,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &GraphFilter::default(),
    )?;
    let slug_to_idx: HashMap<&str, _> = wiki_graph
        .node_indices()
        .map(|idx| (wiki_graph[idx].slug.as_str(), idx))
        .collect();

    if let Some(&root_idx) = slug_to_idx.get(slug.as_str()) {
        // Collect 1-hop and 2-hop neighbors
        let mut hop1: HashSet<petgraph::graph::NodeIndex> = HashSet::new();
        for neighbor in wiki_graph.neighbors_undirected(root_idx) {
            hop1.insert(neighbor);
        }
        for &n1 in &hop1 {
            for n2 in wiki_graph.neighbors_undirected(n1) {
                if n2 == root_idx || hop1.contains(&n2) {
                    continue;
                }
                let node = &wiki_graph[n2];
                if existing_links.contains(&node.slug) {
                    continue;
                }
                let via = &wiki_graph[n1].slug;
                let score = resolved.suggest.graph_neighbor_score;
                let reason = format!("2 hops via {via}");
                candidates
                    .entry(node.slug.clone())
                    .and_modify(|c| {
                        if score > c.score {
                            c.score = score;
                            c.reason = reason.clone();
                        }
                    })
                    .or_insert(CandidateScore {
                        slug: node.slug.clone(),
                        title: node.title.clone(),
                        page_type: node.r#type.clone(),
                        score,
                        reason,
                    });
            }
        }
    }

    // Strategy 3: BM25 similarity (title + summary as query)
    let query = format!("{} {}", input_doc.title, input_doc.summary);
    if !query.trim().is_empty() {
        let results = search::search(
            &query,
            &search::SearchOptions {
                no_excerpt: true,
                top_k: 10,
                ..Default::default()
            },
            &searcher,
            &wiki_name,
            is,
        )?;
        let max_score = results
            .results
            .first()
            .map(|r| r.score)
            .unwrap_or(1.0)
            .max(0.001);
        let bm25_slugs: Vec<&str> = results
            .results
            .iter()
            .filter(|r| r.slug != slug.as_str() && !existing_links.contains(r.slug.as_str()))
            .map(|r| r.slug.as_str())
            .collect();
        let bm25_docs = bulk_fetch_docs(&searcher, is, &bm25_slugs)?;
        for r in &results.results {
            if r.slug == slug.as_str() || existing_links.contains(r.slug.as_str()) {
                continue;
            }
            let score = r.score / max_score * resolved.suggest.bm25_weight;
            let reason = "similar content".to_string();
            candidates
                .entry(r.slug.to_string())
                .and_modify(|c| {
                    if score > c.score {
                        c.score = score;
                        c.reason = reason.clone();
                    }
                })
                .or_insert_with(|| {
                    let page_type = bm25_docs
                        .get(r.slug.as_str())
                        .map(|d| d.page_type.clone())
                        .unwrap_or_else(|| {
                            tracing::warn!(slug = %r.slug, "suggest BM25: doc not found in bulk fetch");
                            String::new()
                        });
                    CandidateScore {
                        slug: r.slug.to_string(),
                        title: r.title.clone(),
                        page_type,
                        score,
                        reason,
                    }
                });
        }
    }

    // Strategy 4: Community peers (same Louvain community, not already linked)
    if let Some(community_map) = get_cached_community_map(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &space.community_cache,
        &searcher,
        resolved.graph.min_nodes_for_communities,
    )? && let Some(&my_community) = community_map.get(slug.as_str())
    {
        let mut peers: Vec<&str> = community_map
            .keys()
            .filter(|s| {
                let ns: &str = s;
                community_map.get(ns).copied() == Some(my_community)
                    && ns != slug.as_str()
                    && !existing_links.contains(ns)
                    && !candidates.contains_key(ns)
            })
            .map(|s| s.as_str())
            .collect();
        peers.sort_unstable();
        let capped_peers: Vec<&str> = peers
            .into_iter()
            .take(resolved.graph.community_suggestions_limit)
            .collect();
        let peer_docs = bulk_fetch_docs(&searcher, is, &capped_peers)?;
        for node_slug in &capped_peers {
            let doc = peer_docs.get(*node_slug).cloned().unwrap_or_default();
            candidates.insert(
                node_slug.to_string(),
                CandidateScore {
                    slug: node_slug.to_string(),
                    title: doc.title,
                    page_type: doc.page_type,
                    score: resolved.suggest.community_peer_score,
                    reason: "same knowledge cluster".to_string(),
                },
            );
        }
    }

    // Rank, filter, cap
    let mut ranked: Vec<CandidateScore> = candidates.into_values().collect();
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.retain(|c| c.score >= min_score);
    ranked.truncate(limit);

    // Build suggestions with edge field
    let suggestions = ranked
        .into_iter()
        .map(|c| {
            let field = suggest_field(&input_type, &c.page_type, &space.type_registry);
            Suggestion {
                uri: format!("wiki://{wiki_name}/{}", c.slug),
                slug: c.slug,
                title: c.title,
                r#type: c.page_type,
                score: (c.score * 100.0).round() / 100.0,
                reason: c.reason,
                field,
            }
        })
        .collect();

    Ok(suggestions)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
struct DocInfo {
    title: String,
    summary: String,
    page_type: String,
    tags: Vec<String>,
    links: Vec<String>,
}

struct CandidateScore {
    slug: String,
    title: String,
    page_type: String,
    score: f32,
    reason: String,
}

fn bulk_fetch_docs(
    searcher: &tantivy::Searcher,
    is: &crate::index_schema::IndexSchema,
    slugs: &[&str],
) -> Result<HashMap<String, DocInfo>> {
    if slugs.is_empty() {
        return Ok(HashMap::new());
    }
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_type = is.field("type");
    let terms: Vec<tantivy::Term> = slugs
        .iter()
        .map(|s| tantivy::Term::from_field_text(f_slug, s))
        .collect();
    let query = tantivy::query::TermSetQuery::new(terms);
    let addrs = searcher.search(&query, &tantivy::collector::DocSetCollector)?;

    let mut map = HashMap::with_capacity(addrs.len());
    for addr in addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(addr)?;
        let slug_val = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug_val.is_empty() {
            continue;
        }
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
        let summary = is
            .try_field("summary")
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tags: Vec<String> = is
            .try_field("tags")
            .map(|f| {
                doc.get_all(f)
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let mut links = Vec::new();
        for field_name in &["sources", "concepts", "body_links", "document_refs"] {
            if let Some(f) = is.try_field(field_name) {
                for val in doc.get_all(f) {
                    if let Some(s) = val.as_str() {
                        links.push(s.to_string());
                    }
                }
            }
        }
        map.insert(
            slug_val,
            DocInfo {
                title,
                summary,
                page_type,
                tags,
                links,
            },
        );
    }
    Ok(map)
}

fn find_doc_by_slug(
    searcher: &tantivy::Searcher,
    is: &crate::index_schema::IndexSchema,
    slug: &str,
) -> Result<DocInfo> {
    Ok(bulk_fetch_docs(searcher, is, &[slug])?
        .remove(slug)
        .unwrap_or_default())
}

fn suggest_field(
    page_type: &str,
    candidate_type: &str,
    registry: &crate::type_registry::SpaceTypeRegistry,
) -> String {
    let source_types = [
        "paper",
        "article",
        "documentation",
        "clipping",
        "transcript",
        "note",
        "data",
        "book-chapter",
        "thread",
    ];
    let is_source = |t: &str| source_types.contains(&t);

    for edge in registry.edges(page_type) {
        let targets = &edge.target_types;
        if targets.iter().any(|t| t == candidate_type) {
            return edge.field.clone();
        }
        if is_source(candidate_type) && targets.iter().any(|t| is_source(t)) {
            return edge.field.clone();
        }
    }

    "[[wikilink]]".to_string()
}
