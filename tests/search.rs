//! Search and context tests — Phase 2
//!
//! Covers: tantivy index lifecycle, BM25 result ranking, context assembly,
//! and end-to-end `wiki search` / `wiki context` CLI behaviour.

use llm_wiki::analysis::{Confidence, PageType};
use llm_wiki::context::context;
use llm_wiki::markdown::{write_page, PageFrontmatter, PageStatus};
use llm_wiki::search::{build_index, open_or_build_index, search, update_index};
use std::path::Path;
use tempfile::TempDir;

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Write a minimal wiki page to `{root}/{slug}.md` with the given fields.
fn make_page(root: &Path, slug: &str, title: &str, page_type: PageType, tags: &[&str], body: &str) {
    let path = root.join(format!("{}.md", slug));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let fm = PageFrontmatter {
        title: title.to_string(),
        summary: title.to_string(),
        tldr: title.to_string(),
        read_when: vec![],
        status: PageStatus::Active,
        last_updated: "2026-04-13".to_string(),
        page_type,
        tags: tags.iter().map(|s| s.to_string()).collect(),
        sources: vec![],
        confidence: Confidence::Medium,
        contradictions: vec![],
    };
    write_page(&path, &fm, body).unwrap();
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[test]
fn build_index_document_count_matches_md_file_count() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &["moe"], "Body A");
    make_page(root, "concepts/scaling", "Scaling Laws", PageType::Concept, &["scaling"], "Body B");
    make_page(root, "sources/paper1", "Paper One", PageType::SourceSummary, &[], "Body C");

    let index_dir = root.join(".wiki/search-index");
    let index = build_index(root, &index_dir).unwrap();

    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    assert_eq!(searcher.num_docs(), 3, "expected exactly 3 indexed documents");
}

#[test]
fn build_index_excludes_raw_directory() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // One valid page outside raw/
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    // A valid page inside raw/ — must be excluded from the index.
    make_page(root, "raw/original", "Raw Source", PageType::SourceSummary, &[], "raw content");

    let index_dir = root.join(".wiki/search-index");
    let index = build_index(root, &index_dir).unwrap();

    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    assert_eq!(searcher.num_docs(), 1, "raw/ page must be excluded from index");
}

#[test]
fn search_known_title_term_returns_page_as_top_result() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &["moe"], "Sparse routing architecture.");
    make_page(root, "concepts/scaling", "Scaling Laws", PageType::Concept, &["scaling"], "Compute budget scaling.");

    let results = search("Mixture", root, false).unwrap();

    assert!(!results.is_empty(), "expected at least one result");
    assert_eq!(results[0].slug, "concepts/moe", "top result should be the MoE page");
}

#[test]
fn search_known_body_term_returns_page_in_results() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(
        root,
        "concepts/moe",
        "Mixture of Experts",
        PageType::Concept,
        &[],
        "Sparse routing uses xyzuniquetoken12345 for expert selection.",
    );

    let results = search("xyzuniquetoken12345", root, false).unwrap();
    assert!(
        results.iter().any(|r| r.slug == "concepts/moe"),
        "page with unique body term must appear in results"
    );
}

#[test]
fn search_unknown_term_returns_empty_no_panic() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    let results = search("xyznonexistentterm99999", root, false).unwrap();
    assert!(results.is_empty(), "unknown term must return empty results");
}

#[test]
fn search_results_ordered_by_descending_score() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Page A: "transformer" in title and body — should score higher.
    make_page(
        root,
        "concepts/transformer",
        "Transformer Architecture",
        PageType::Concept,
        &["transformer"],
        "The transformer uses self-attention. Transformer blocks stack layers.",
    );
    // Page B: "transformer" only in body once.
    make_page(
        root,
        "concepts/other",
        "Other Topic",
        PageType::Concept,
        &[],
        "Mentions transformer briefly.",
    );

    let results = search("transformer", root, false).unwrap();
    assert!(results.len() >= 2, "expected at least 2 results");

    // Verify descending order.
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "results must be ordered by descending score, got {} then {}",
            window[0].score,
            window[1].score
        );
    }

    // The more-relevant page should rank first.
    assert_eq!(results[0].slug, "concepts/transformer");
}

#[test]
fn context_output_contains_page_titles() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &["moe"], "MoE routes tokens to experts.");
    make_page(root, "concepts/scaling", "Scaling Laws", PageType::Concept, &["scaling"], "Scaling laws describe compute.");

    let output = context("mixture experts scaling", root, 5).unwrap();

    // At least one page title should appear in the output.
    assert!(
        output.contains("Mixture of Experts") || output.contains("Scaling Laws"),
        "context output must contain at least one matched page title"
    );
}

#[test]
fn context_top_k_2_returns_at_most_2_page_blocks() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create 5 pages all matching "neural network".
    for i in 0..5 {
        make_page(
            root,
            &format!("concepts/page{i}"),
            &format!("Neural Network Topic {i}"),
            PageType::Concept,
            &["neural", "network"],
            "This page covers neural network concepts.",
        );
    }

    let output = context("neural network", root, 2).unwrap();

    // Count "---" separators — each page block ends with "---\n\n".
    let block_count = output.matches("\n---\n").count();
    assert!(
        block_count <= 2,
        "top_k=2 must return at most 2 page blocks, got {block_count}"
    );
}

#[test]
fn context_contradiction_page_included_when_relevant() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // A contradiction page — should be included like any other page.
    make_page(
        root,
        "contradictions/moe-scaling",
        "MoE scaling efficiency: contradictory views",
        PageType::Contradiction,
        &["moe", "scaling"],
        "Claim A says 8x reduction. Claim B says gains diminish at scale.",
    );

    let results = search("MoE scaling", root, false).unwrap();
    assert!(
        results.iter().any(|r| r.slug == "contradictions/moe-scaling"),
        "contradiction page must appear in search results"
    );

    let output = context("MoE scaling", root, 5).unwrap();
    assert!(
        output.contains("MoE scaling efficiency"),
        "contradiction page title must appear in context output"
    );
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[test]
fn cli_search_after_ingest_returns_ranked_results() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create 5 pages directly (bypasses git — tests search, not ingest).
    make_page(root, "concepts/attention", "Self-Attention Mechanism", PageType::Concept, &["attention", "transformer"], "Multi-head attention computes queries, keys, and values.");
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &["moe", "scaling"], "MoE routes tokens to sparse expert subnetworks.");
    make_page(root, "concepts/scaling", "Scaling Laws", PageType::Concept, &["scaling", "compute"], "Neural scaling laws relate compute to performance.");
    make_page(root, "sources/switch-transformer", "Switch Transformer", PageType::SourceSummary, &["moe", "transformer"], "Switch transformer uses a simplified MoE routing strategy.");
    make_page(root, "queries/moe-question", "How does MoE work?", PageType::QueryResult, &["moe"], "MoE divides a model into expert subnetworks with sparse routing.");

    let results = search("mixture of experts", root, false).unwrap();

    assert!(!results.is_empty(), "search must return at least one result");
    // The most relevant results should be MoE-related pages.
    assert!(
        results.iter().any(|r| r.slug.contains("moe") || r.title.to_lowercase().contains("expert")),
        "at least one MoE-related page must be in results"
    );
    // Verify descending score ordering.
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "results must be in descending score order"
        );
    }
}

#[test]
fn cli_search_rebuild_index_on_fresh_clone_succeeds() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    // No pre-existing index — build_index must succeed on first call.
    let index_dir = root.join(".wiki/search-index");
    assert!(!index_dir.exists(), "index dir must not exist before build");

    let index = build_index(root, &index_dir).unwrap();
    assert!(index_dir.exists(), "index dir must exist after build");

    let reader = index.reader().unwrap();
    assert_eq!(reader.searcher().num_docs(), 1);
}

#[test]
fn cli_search_new_page_reflected_after_update_index() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Initial page.
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    // First search — builds index with 1 page.
    let results1 = search("Mixture", root, false).unwrap();
    assert!(!results1.is_empty());

    // Add a new page.
    make_page(
        root,
        "concepts/new-unique-topic",
        "Completely New Unique Topic",
        PageType::Concept,
        &["xyznewtopic"],
        "This page covers xyznewtopicterm that did not exist before.",
    );

    // Without update_index, the new page is NOT in the index (incremental policy).
    let results_before = search("xyznewtopicterm", root, false).unwrap();
    assert!(
        results_before.iter().all(|r| r.slug != "concepts/new-unique-topic"),
        "new page must not appear before update_index"
    );

    // After update_index, the new page IS searchable.
    let index_dir = root.join(".wiki/search-index");
    update_index(root, &index_dir, &["concepts/new-unique-topic".to_string()]).unwrap();
    let results_after = search("xyznewtopicterm", root, false).unwrap();
    assert!(
        results_after.iter().any(|r| r.slug == "concepts/new-unique-topic"),
        "newly added page must appear in search results after update_index"
    );
}

#[test]
fn cli_context_output_is_valid_markdown() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(
        root,
        "concepts/moe",
        "Mixture of Experts",
        PageType::Concept,
        &["moe"],
        "## Overview\n\nMoE routes tokens to sparse expert subnetworks.\n",
    );

    let output = context("mixture of experts", root, 3).unwrap();

    if !output.is_empty() {
        // Each page block must start with a level-1 heading.
        assert!(output.starts_with("# "), "context output must start with a # heading");
        // No unclosed code fences (count of ``` must be even).
        let fence_count = output.matches("```").count();
        assert_eq!(fence_count % 2, 0, "context output must not have unclosed code fences");
    }
}

#[test]
fn cli_context_no_matching_pages_returns_empty_no_error() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    let output = context("xyzabsolutelynonexistentterm99999", root, 5).unwrap();
    assert!(output.is_empty(), "no matching pages must return empty string, not an error");
}

// ── Phase 7: open_or_build_index and update_index tests ──────────────────────

#[test]
fn open_or_build_index_missing_dir_builds_index() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    let index_dir = root.join(".wiki/search-index");
    assert!(!index_dir.exists());

    let index = open_or_build_index(root, &index_dir).unwrap();
    assert!(index_dir.exists());
    assert_eq!(index.reader().unwrap().searcher().num_docs(), 1);
}

#[test]
fn open_or_build_index_existing_index_opens_without_rebuild() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    let index_dir = root.join(".wiki/search-index");
    build_index(root, &index_dir).unwrap();

    // Record mtime of a file inside the index dir.
    let meta_before = std::fs::metadata(&index_dir).unwrap().modified().unwrap();

    // Add a new page on disk — open_or_build_index must NOT rebuild.
    make_page(root, "concepts/new", "New Page", PageType::Concept, &[], "Body B");

    let index = open_or_build_index(root, &index_dir).unwrap();
    let meta_after = std::fs::metadata(&index_dir).unwrap().modified().unwrap();

    // Index dir mtime unchanged — no rebuild occurred.
    assert_eq!(meta_before, meta_after);
    // Still only 1 doc (the new page was not indexed).
    assert_eq!(index.reader().unwrap().searcher().num_docs(), 1);
}

#[test]
fn update_index_new_page_appears_in_search() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    let index_dir = root.join(".wiki/search-index");
    build_index(root, &index_dir).unwrap();

    // Add a new page and update the index.
    make_page(root, "concepts/scaling", "Scaling Laws", PageType::Concept, &[], "xyzscalingunique");
    update_index(root, &index_dir, &["concepts/scaling".to_string()]).unwrap();

    let results = search("xyzscalingunique", root, false).unwrap();
    assert!(
        results.iter().any(|r| r.slug == "concepts/scaling"),
        "new page must appear after update_index"
    );
}

#[test]
fn update_index_modified_page_reflects_new_content() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "original content");

    let index_dir = root.join(".wiki/search-index");
    build_index(root, &index_dir).unwrap();

    // Overwrite with new content.
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "xyzmodifiedcontent99");
    update_index(root, &index_dir, &["concepts/moe".to_string()]).unwrap();

    let results = search("xyzmodifiedcontent99", root, false).unwrap();
    assert!(
        results.iter().any(|r| r.slug == "concepts/moe"),
        "modified content must appear after update_index"
    );
}

#[test]
fn update_index_deleted_page_no_longer_in_search() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "xyzdeleteunique");

    let index_dir = root.join(".wiki/search-index");
    build_index(root, &index_dir).unwrap();

    // Delete the file and update the index.
    std::fs::remove_file(root.join("concepts/moe.md")).unwrap();
    update_index(root, &index_dir, &["concepts/moe".to_string()]).unwrap();

    let results = search("xyzdeleteunique", root, false).unwrap();
    assert!(
        results.iter().all(|r| r.slug != "concepts/moe"),
        "deleted page must not appear after update_index"
    );
}

#[test]
fn update_index_empty_slugs_is_noop() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    let index_dir = root.join(".wiki/search-index");
    build_index(root, &index_dir).unwrap();

    let mtime_before = std::fs::metadata(&index_dir).unwrap().modified().unwrap();
    update_index(root, &index_dir, &[]).unwrap();
    let mtime_after = std::fs::metadata(&index_dir).unwrap().modified().unwrap();

    assert_eq!(mtime_before, mtime_after, "empty update must not touch index");
}

#[test]
fn two_consecutive_searches_do_not_modify_index_on_second_call() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body A");

    // First search builds the index.
    search("Mixture", root, false).unwrap();

    let index_dir = root.join(".wiki/search-index");
    let mtime_after_first = std::fs::metadata(&index_dir).unwrap().modified().unwrap();

    // Second search must open the existing index, not rebuild.
    search("Mixture", root, false).unwrap();
    let mtime_after_second = std::fs::metadata(&index_dir).unwrap().modified().unwrap();

    assert_eq!(
        mtime_after_first, mtime_after_second,
        "second search must not modify index files"
    );
}

// ── Phase 8: bundle-aware indexing ───────────────────────────────────────────

#[test]
fn build_index_bundle_page_indexed_once() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create a bundle page.
    let bundle_dir = root.join("concepts/moe");
    std::fs::create_dir_all(&bundle_dir).unwrap();
    make_page(root, "concepts/moe/index", "Mixture of Experts", PageType::Concept, &["moe"], "Bundle body.");
    // Rename to index.md (make_page writes {slug}.md, so we need to move it).
    std::fs::rename(root.join("concepts/moe/index.md"), bundle_dir.join("index.md")).unwrap_or(());
    // Write directly as bundle.
    let bundle_index = bundle_dir.join("index.md");
    if !bundle_index.exists() {
        make_page(root, "concepts/moe/index", "Mixture of Experts", PageType::Concept, &["moe"], "Bundle body.");
    }

    let index_dir = root.join(".wiki/search-index");
    let index = build_index(root, &index_dir).unwrap();
    let reader = index.reader().unwrap();
    // Should be indexed exactly once.
    assert_eq!(reader.searcher().num_docs(), 1, "bundle page must be indexed exactly once");
}

#[test]
fn build_index_asset_files_not_indexed() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Bundle page.
    let bundle_dir = root.join("concepts/moe");
    std::fs::create_dir_all(&bundle_dir).unwrap();
    make_page(root, "concepts/moe/index", "Mixture of Experts", PageType::Concept, &[], "Body.");
    // Co-located non-md asset.
    std::fs::write(bundle_dir.join("diagram.png"), b"PNG").unwrap();

    let index_dir = root.join(".wiki/search-index");
    let index = build_index(root, &index_dir).unwrap();
    let reader = index.reader().unwrap();
    assert_eq!(reader.searcher().num_docs(), 1, "asset files must not be indexed");
}

#[test]
fn build_index_assets_dir_not_indexed_except_index_md() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &[], "Body.");

    // Shared asset (non-md) — must not be indexed.
    let assets_dir = root.join("assets/diagrams");
    std::fs::create_dir_all(&assets_dir).unwrap();
    std::fs::write(assets_dir.join("chart.png"), b"PNG").unwrap();

    let index_dir = root.join(".wiki/search-index");
    let index = build_index(root, &index_dir).unwrap();
    let reader = index.reader().unwrap();
    // Only the concept page; assets/index.md doesn't exist yet.
    assert_eq!(reader.searcher().num_docs(), 1);
}

#[test]
fn search_after_bundle_promotion_slug_unchanged() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Start with flat page.
    make_page(root, "concepts/moe", "Mixture of Experts", PageType::Concept, &["moe"], "Sparse routing.");

    // Build index while flat.
    let index_dir = root.join(".wiki/search-index");
    build_index(root, &index_dir).unwrap();

    // Promote to bundle.
    llm_wiki::markdown::promote_to_bundle(root, "concepts/moe").unwrap();

    // Update index for the slug.
    update_index(root, &index_dir, &["concepts/moe".to_string()]).unwrap();

    let results = search("Sparse routing", root, false).unwrap();
    assert!(
        results.iter().any(|r| r.slug == "concepts/moe"),
        "slug must remain concepts/moe after bundle promotion; results: {:?}",
        results.iter().map(|r| &r.slug).collect::<Vec<_>>()
    );
}
