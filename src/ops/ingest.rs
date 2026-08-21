use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;

use crate::engine::{EngineState, WikiEngine};
use crate::git;
use crate::ingest;

/// Ingest a path without redaction; delegates to `ingest_with_redact`.
pub fn ingest(
    engine: &EngineState,
    manager: &WikiEngine,
    path: &str,
    dry_run: bool,
    wiki_name: &str,
) -> Result<ingest::IngestReport> {
    ingest_with_redact(engine, manager, path, dry_run, false, wiki_name)
}

/// Ingest a path with optional redaction pass, incremental index refresh, and edge validation.
pub fn ingest_with_redact(
    engine: &EngineState,
    manager: &WikiEngine,
    path: &str,
    dry_run: bool,
    redact: bool,
    wiki_name: &str,
) -> Result<ingest::IngestReport> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    // Build changed-paths set from git diff (normal ingest only; dry_run validates all).
    // Paths from collect_changed_files are relative to repo_root; strip the wiki prefix
    // so they match paths relative to wiki_root used inside the walk loop.
    let changed_paths = if dry_run {
        None
    } else {
        let last = space.index_manager.last_commit();
        let wiki_rel = space
            .wiki_root
            .strip_prefix(&space.repo_root)
            .unwrap_or(&space.wiki_root);
        match git::collect_changed_files(&space.repo_root, &space.wiki_root, last.as_deref()) {
            Ok(map) => {
                let set: HashSet<_> = map
                    .into_keys()
                    .filter_map(|p| p.strip_prefix(wiki_rel).map(|r| r.to_path_buf()).ok())
                    .collect();
                Some(set)
            }
            Err(e) => {
                tracing::warn!(error = %e, "git diff failed; falling back to full re-index");
                None
            }
        }
    };

    let redact_cfg = if redact {
        Some(resolved.redact.clone())
    } else {
        None
    };

    let opts = ingest::IngestOptions {
        dry_run,
        auto_commit: resolved.ingest.auto_commit,
        changed_paths,
        redact: redact_cfg,
    };
    let mut report = ingest::ingest(
        Path::new(path),
        &opts,
        &space.wiki_root,
        &space.type_registry,
        &resolved.validation,
    )?;

    if !dry_run {
        manager.refresh_index(wiki_name)?;

        // Validate edge targets after index update (targets must be indexed)
        let edge_warnings = validate_edge_targets(space)?;
        report.warnings.extend(edge_warnings);
    }

    Ok(report)
}

struct PageEdgeData {
    slug: String,
    page_type: String,
    edge_targets: Vec<(String, Vec<String>)>,
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
    if top_docs.len() >= 100_000 {
        tracing::warn!(
            count = top_docs.len(),
            "ingest dedup query hit 100 000 page limit — existing pages beyond limit may be re-indexed"
        );
    }

    let mut slug_types: std::collections::HashMap<String, String> =
        std::collections::HashMap::with_capacity(top_docs.len());
    let mut pages: Vec<PageEdgeData> = Vec::with_capacity(top_docs.len());

    // Single pass: build slug_types and collect edge targets simultaneously
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
        if slug.is_empty() {
            continue;
        }
        slug_types.insert(slug.clone(), page_type.clone());

        let edge_targets: Vec<(String, Vec<String>)> = space
            .type_registry
            .edges(&page_type)
            .iter()
            .filter(|decl| !decl.target_types.is_empty())
            .filter_map(|decl| {
                let field_handle = is.try_field(&decl.field)?;
                let targets: Vec<String> = doc
                    .get_all(field_handle)
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if targets.is_empty() {
                    None
                } else {
                    Some((decl.field.clone(), targets))
                }
            })
            .collect();

        pages.push(PageEdgeData {
            slug,
            page_type,
            edge_targets,
        });
    }

    // In-memory pass: check edges against the now-complete slug_types map
    let mut warnings = Vec::new();
    for page in &pages {
        for (field_name, targets) in &page.edge_targets {
            let decl = space
                .type_registry
                .edges(&page.page_type)
                .iter()
                .find(|d| &d.field == field_name);
            let Some(decl) = decl else { continue };
            for target in targets {
                if let Some(target_type) = slug_types.get(target.as_str())
                    && !decl.target_types.contains(target_type)
                {
                    warnings.push(format!(
                        "{}: edge '{}' target '{}' has type '{}', expected one of {:?}",
                        page.slug, decl.relation, target, target_type, decl.target_types
                    ));
                }
            }
        }
    }

    Ok(warnings)
}
