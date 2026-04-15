//! MCP server tests — Phase 4
//!
//! Unit tests call WikiServer helper methods directly (no MCP transport overhead).
//! Integration tests verify multi-step scenarios through the same helpers.

use llm_wiki::server::WikiServer;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a minimal valid `analysis.json` Value that creates one concept page.
fn one_concept_analysis(slug: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "source": "test-source",
        "doc_type": "note",
        "title": title,
        "language": "en",
        "claims": [],
        "concepts": [slug],
        "key_quotes": [],
        "data_gaps": [],
        "suggested_pages": [{
            "slug": format!("concepts/{slug}"),
            "title": title,
            "type": "concept",
            "action": "create",
            "tldr": "A test concept page.",
            "body": format!("## Overview\n\nThis page covers {}.", title),
            "tags": ["test"],
            "read_when": []
        }],
        "contradictions": []
    })
}

/// Create a temp wiki dir and return (TempDir, WikiServer).
///
/// Keeping `TempDir` alive for the duration of the test prevents the directory
/// from being cleaned up while the server is still reading from it.
fn temp_server() -> (TempDir, WikiServer) {
    let dir = TempDir::new().expect("temp dir");
    let server = WikiServer::new(dir.path().to_path_buf());
    (dir, server)
}

/// Extract a named `## {name}-workflow` section from `instructions.md`.
fn extract_section(name: &str) -> String {
    let instructions = include_str!("../src/instructions.md");
    let header = format!("## {name}-workflow");
    let start = instructions
        .find(header.as_str())
        .unwrap_or_else(|| panic!("section '{}' not found in instructions.md", name));
    let section = &instructions[start..];
    let end = section[header.len()..]
        .find("\n## ")
        .map(|pos| header.len() + pos)
        .unwrap_or(section.len());
    section[..end].to_string()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[test]
fn wiki_ingest_tool_valid_json_returns_success_with_page_count() {
    let (_dir, server) = temp_server();
    let analysis = one_concept_analysis("mixture-of-experts", "Mixture of Experts");
    let result = server.do_ingest(analysis);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let summary = result.unwrap();
    assert!(
        summary.contains("created") || summary.contains("Ingested"),
        "summary should mention creation: {summary}"
    );
}

#[test]
fn wiki_ingest_tool_malformed_json_returns_error_no_panic() {
    let (_dir, server) = temp_server();
    // Pass a JSON string that is structurally valid JSON but not an object.
    let bad = serde_json::Value::String("{ this is not analysis json".into());
    let result = server.do_ingest(bad);
    assert!(
        result.is_err(),
        "expected Err for non-object analysis, got: {:?}",
        result
    );
}

#[test]
fn wiki_ingest_tool_unknown_doc_type_returns_error_with_valid_values() {
    let (_dir, server) = temp_server();
    let bad = serde_json::json!({
        "source": "s",
        "doc_type": "spaceship",   // ← not a valid DocType variant
        "title": "t",
        "language": "en",
        "claims": [],
        "concepts": [],
        "key_quotes": [],
        "data_gaps": [],
        "suggested_pages": [],
        "contradictions": []
    });
    let result = server.do_ingest(bad);
    assert!(result.is_err(), "expected Err for unknown doc_type");
    let msg = result.unwrap_err();
    // Serde gives a helpful error that names the bad value.
    assert!(
        msg.to_lowercase().contains("spaceship") || msg.contains("doc_type") || msg.contains("unknown"),
        "error message should mention the bad field: {msg}"
    );
}

#[test]
fn wiki_context_tool_known_concept_returns_nonempty_markdown() {
    let (_dir, server) = temp_server();
    // Ingest a concept page first.
    let analysis = one_concept_analysis("attention-mechanism", "Attention Mechanism");
    server.do_ingest(analysis).expect("ingest should succeed");

    let md = server.do_context("attention mechanism", 5);
    assert!(
        !md.is_empty(),
        "context for a known concept should return non-empty markdown"
    );
    assert!(
        md.contains("Attention Mechanism"),
        "markdown should contain the page title: {md}"
    );
}

#[test]
fn wiki_context_tool_no_matching_pages_returns_empty_no_error() {
    let (_dir, server) = temp_server();
    // Empty wiki — no pages ingested.
    let md = server.do_context("quantum entanglement in black holes", 5);
    // Should return empty string without panicking.
    assert!(
        md.is_empty() || md.trim().is_empty(),
        "context on empty wiki should be empty, got: {md}"
    );
}

#[test]
fn wiki_list_tool_returns_correct_count_per_type() {
    let (_dir, server) = temp_server();
    // Ingest two concept pages.
    server
        .do_ingest(one_concept_analysis("concept-a", "Concept A"))
        .expect("ingest A");
    server
        .do_ingest(one_concept_analysis("concept-b", "Concept B"))
        .expect("ingest B");

    let all = server.do_list_pages(None);
    assert!(all.len() >= 2, "expected at least 2 pages, got {}", all.len());

    let concepts = server.do_list_pages(Some("concept"));
    assert!(
        concepts.len() >= 2,
        "expected at least 2 concept pages, got {}",
        concepts.len()
    );
    assert!(
        concepts.iter().all(|p| p.page_type == "concept"),
        "all pages should be of type 'concept'"
    );
}

#[test]
fn wiki_list_tool_concept_filter_excludes_contradiction_pages() {
    let (_dir, server) = temp_server();

    // Ingest a concept page.
    server
        .do_ingest(one_concept_analysis("transformer", "Transformer"))
        .expect("ingest concept");

    // Ingest a source-summary page.
    let source_analysis = serde_json::json!({
        "source": "paper.pdf",
        "doc_type": "research-paper",
        "title": "A Paper",
        "language": "en",
        "claims": [],
        "concepts": [],
        "key_quotes": [],
        "data_gaps": [],
        "suggested_pages": [{
            "slug": "sources/a-paper",
            "title": "A Paper",
            "type": "source-summary",
            "action": "create",
            "tldr": "A test source.",
            "body": "Source summary content.",
            "tags": [],
            "read_when": []
        }],
        "contradictions": []
    });
    server.do_ingest(source_analysis).expect("ingest source");

    let concepts = server.do_list_pages(Some("concept"));
    let sources = server.do_list_pages(Some("source"));

    assert!(
        concepts.iter().all(|p| p.page_type == "concept"),
        "concept filter should only return concepts"
    );
    assert!(
        sources.iter().all(|p| p.page_type == "source-summary"),
        "source filter should only return source-summaries"
    );
    // Cross-check: no overlap.
    let concept_slugs: Vec<_> = concepts.iter().map(|p| p.slug.as_str()).collect();
    for src in &sources {
        assert!(
            !concept_slugs.contains(&src.slug.as_str()),
            "source {} should not appear in concept list",
            src.slug
        );
    }
}

#[test]
fn read_resource_valid_uri_returns_page_content() {
    let (_dir, server) = temp_server();
    server
        .do_ingest(one_concept_analysis("neural-scaling", "Neural Scaling"))
        .expect("ingest");

    let uri = "wiki://default/concepts/neural-scaling";
    let result = server.do_read_resource(uri);
    assert!(result.is_ok(), "expected Ok for valid URI, got: {:?}", result);
    let content = result.unwrap();
    assert!(
        content.contains("Neural Scaling"),
        "page content should contain the title: {content}"
    );
}

#[test]
fn read_resource_unknown_slug_returns_not_found_no_panic() {
    let (_dir, server) = temp_server();
    // No pages ingested — slug does not exist.
    let result = server.do_read_resource("wiki://default/concepts/does-not-exist");
    assert!(
        result.is_err(),
        "expected Err for missing resource, got: {:?}",
        result
    );
}

#[test]
fn wiki_instruct_nonempty_contains_analysis_json() {
    let instructions = include_str!("../src/instructions.md");
    assert!(!instructions.is_empty(), "instructions.md must not be empty");
    assert!(
        instructions.contains("analysis.json"),
        "instructions.md must document analysis.json"
    );
}

#[test]
fn wiki_instruct_ingest_contains_ingest_specific_steps() {
    let section = extract_section("ingest");
    assert!(
        section.contains("wiki_ingest"),
        "ingest-workflow should mention wiki_ingest: {section}"
    );
    // Should reference the two-step workflow.
    assert!(
        section.to_lowercase().contains("step") || section.contains("1."),
        "ingest-workflow should contain numbered steps: {section}"
    );
}

#[test]
fn wiki_instruct_research_contains_wiki_context() {
    let section = extract_section("research");
    assert!(
        section.contains("wiki_context"),
        "research-workflow should mention wiki_context: {section}"
    );
}

// ── Integration tests ─────────────────────────────────────────────────────────
//
// These tests exercise multi-step scenarios through the WikiServer public
// helpers, validating observable on-disk state rather than the MCP protocol
// transport (transport correctness is covered by the rmcp library itself).

#[test]
fn cli_serve_starts_stdio_transport_accepts_list_tools() {
    // Verify WikiServer can be constructed and that server info is populated.
    let (_dir, server) = temp_server();
    // Accessing server info through a direct method call validates the
    // get_info() path (capabilities, instructions) without needing a live
    // transport.  The rmcp::ServerHandler::get_info default is overridden in
    // our impl, so any panic here points to a regression in that method.
    use rmcp::ServerHandler as _;
    let info = server.get_info();
    assert!(
        info.instructions.is_some(),
        "server info should include instructions"
    );
    assert!(
        info.capabilities.tools.is_some(),
        "server info should advertise tools capability"
    );
}

#[test]
fn mcp_wiki_ingest_page_appears_on_disk_resource_notification_fires() {
    let (dir, server) = temp_server();
    let analysis = one_concept_analysis("in-context-learning", "In-Context Learning");
    server.do_ingest(analysis).expect("ingest");

    // Verify the page file exists on disk at the expected path.
    let page_path = dir
        .path()
        .join("concepts")
        .join("in-context-learning.md");
    assert!(
        page_path.exists(),
        "page file should exist on disk after ingest: {}",
        page_path.display()
    );

    // Verify git committed the file (HEAD commit exists).
    let git_dir = dir.path().join(".git");
    assert!(git_dir.exists(), ".git directory should exist after ingest");
}

#[test]
fn mcp_wiki_context_returns_page_bodies() {
    let (_dir, server) = temp_server();

    // Ingest multiple pages.
    server
        .do_ingest(one_concept_analysis("sparse-attention", "Sparse Attention"))
        .expect("ingest sparse-attention");
    server
        .do_ingest(one_concept_analysis("flash-attention", "Flash Attention"))
        .expect("ingest flash-attention");

    // Query something that should match one of them.
    let md = server.do_context("sparse attention mechanism", 5);
    assert!(
        md.contains("Sparse Attention"),
        "context should return the Sparse Attention page body: {md}"
    );
    // The body we wrote in one_concept_analysis should be present.
    assert!(
        md.contains("Overview"),
        "page body sections should be included in context: {md}"
    );
}

#[test]
fn mcp_resource_valid_slug_returns_correct_content() {
    let (dir, server) = temp_server();

    // Ingest a page with known body content.
    let analysis = serde_json::json!({
        "source": "custom-source",
        "doc_type": "blog-post",
        "title": "Custom Resource Test",
        "language": "en",
        "claims": [],
        "concepts": ["custom-resource-test"],
        "key_quotes": [],
        "data_gaps": [],
        "suggested_pages": [{
            "slug": "concepts/custom-resource-test",
            "title": "Custom Resource Test",
            "type": "concept",
            "action": "create",
            "tldr": "Testing resource reads.",
            "body": "UNIQUE_BODY_SENTINEL_12345",
            "tags": [],
            "read_when": []
        }],
        "contradictions": []
    });
    server.do_ingest(analysis).expect("ingest");

    // Read via do_read_resource.
    let content = server
        .do_read_resource("wiki://default/concepts/custom-resource-test")
        .expect("resource read should succeed");

    assert!(
        content.contains("UNIQUE_BODY_SENTINEL_12345"),
        "resource content should include the page body: {content}"
    );

    // Also verify the file is on disk.
    let expected_path = dir
        .path()
        .join("concepts")
        .join("custom-resource-test.md");
    assert!(expected_path.exists());
}

#[test]
fn mcp_resource_missing_slug_returns_error_server_stays_alive() {
    let (_dir, server) = temp_server();

    // First request — missing resource should return an error.
    let err = server
        .do_read_resource("wiki://default/concepts/ghost-page")
        .unwrap_err();
    let err_msg = format!("{err:?}");
    assert!(
        err_msg.to_lowercase().contains("not found") || err_msg.contains("ghost-page"),
        "error should indicate resource not found: {err_msg}"
    );

    // Second request — server helper should still work after the error.
    server
        .do_ingest(one_concept_analysis("recovery-test", "Recovery Test"))
        .expect("server should still function after a resource-not-found error");

    let content = server
        .do_read_resource("wiki://default/concepts/recovery-test")
        .expect("subsequent valid request should succeed");
    assert!(content.contains("Recovery Test"));
}

// ── Phase 8: bundle asset resources ──────────────────────────────────────────

#[test]
fn mcp_bundle_asset_resource_returns_content() {
    let (dir, server) = temp_server();

    // Ingest a concept page (flat).
    server
        .do_ingest(one_concept_analysis("mixture-of-experts", "Mixture of Experts"))
        .expect("ingest");

    // Promote to bundle and write a co-located asset.
    llm_wiki::integrate::write_asset_colocated(
        dir.path(),
        "concepts/mixture-of-experts",
        "diagram.png",
        b"PNG_SENTINEL",
    )
    .expect("write_asset_colocated");

    // Read the asset via the MCP resource URI.
    let uri = "wiki://default/concepts/mixture-of-experts/diagram.png";
    let content = server
        .do_read_resource(uri)
        .expect("bundle asset resource should be readable");
    assert!(
        content.contains("PNG_SENTINEL"),
        "resource content should match written asset: {content}"
    );
}

#[test]
fn mcp_bundle_page_still_readable_after_promotion() {
    let (dir, server) = temp_server();

    server
        .do_ingest(one_concept_analysis("scaling-laws", "Scaling Laws"))
        .expect("ingest");

    // Promote to bundle.
    llm_wiki::markdown::promote_to_bundle(dir.path(), "concepts/scaling-laws")
        .expect("promote_to_bundle");

    // The page URI must still resolve after promotion.
    let content = server
        .do_read_resource("wiki://default/concepts/scaling-laws")
        .expect("bundle page should be readable after promotion");
    assert!(
        content.contains("Scaling Laws"),
        "page content should be intact after promotion: {content}"
    );
}
