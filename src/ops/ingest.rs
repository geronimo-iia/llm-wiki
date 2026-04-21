use std::path::Path;

use anyhow::Result;

use crate::engine::{EngineState, WikiEngine};
use crate::ingest;

pub fn ingest(
    engine: &EngineState,
    manager: &WikiEngine,
    path: &str,
    dry_run: bool,
    wiki_name: &str,
) -> Result<ingest::IngestReport> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let opts = ingest::IngestOptions {
        dry_run,
        auto_commit: resolved.ingest.auto_commit,
    };
    let mut report = ingest::ingest(
        Path::new(path),
        &opts,
        &space.wiki_root,
        &space.type_registry,
        &resolved.validation,
    )?;

    if !dry_run {
        if let Err(e) = manager.refresh_index(wiki_name) {
            tracing::warn!(error = %e, "incremental index update failed after ingest");
        }

        // Validate edge targets after index update (targets must be indexed)
        let edge_warnings = validate_edge_targets(space)?;
        report.warnings.extend(edge_warnings);
    }

    Ok(report)
}

fn validate_edge_targets(space: &crate::engine::SpaceContext) -> Result<Vec<String>> {
    use tantivy::schema::Value;

    let searcher = space.index_manager.searcher()?;
    let is = &space.index_schema;
    let f_slug = is.field("slug");
    let f_type = is.field("type");

    // Build a slug→type map from the index
    let top_docs = searcher.search(
        &tantivy::query::AllQuery,
        &tantivy::collector::TopDocs::with_limit(100_000).order_by_score(),
    )?;
    let mut slug_types: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (_score, doc_addr) in &top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_addr)?;
        let slug = doc.get_first(f_slug).and_then(|v| v.as_str()).unwrap_or("");
        let page_type = doc.get_first(f_type).and_then(|v| v.as_str()).unwrap_or("");
        if !slug.is_empty() {
            slug_types.insert(slug.to_string(), page_type.to_string());
        }
    }

    let mut warnings = Vec::new();

    // For each page, check edge targets
    for (_score, doc_addr) in &top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        for decl in space.type_registry.edges(&page_type) {
            if decl.target_types.is_empty() {
                continue;
            }
            if let Some(field_handle) = is.try_field(&decl.field) {
                for val in doc.get_all(field_handle) {
                    if let Some(target) = val.as_str() {
                        if let Some(target_type) = slug_types.get(target) {
                            if !decl.target_types.contains(target_type) {
                                warnings.push(format!(
                                    "{}: edge '{}' target '{}' has type '{}', expected one of {:?}",
                                    slug, decl.relation, target, target_type, decl.target_types
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(warnings)
}
