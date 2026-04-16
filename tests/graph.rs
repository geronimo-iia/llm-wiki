use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::graph::*;

fn setup_repo(dir: &Path) -> std::path::PathBuf {
    let wiki_root = dir.join("wiki");
    fs::create_dir_all(&wiki_root).unwrap();
    fs::create_dir_all(dir.join("inbox")).unwrap();
    fs::create_dir_all(dir.join("raw")).unwrap();
    git::init_repo(dir).unwrap();
    fs::write(dir.join("README.md"), "# test\n").unwrap();
    git::commit(dir, "init").unwrap();
    wiki_root
}

fn write_page(wiki_root: &Path, rel_path: &str, content: &str) {
    let path = wiki_root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn page_with_sources(title: &str, sources: &[&str]) -> String {
    let mut fm = format!(
        "---\ntitle: \"{title}\"\nsummary: \"s\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: concept\n"
    );
    if !sources.is_empty() {
        fm.push_str("sources:\n");
        for s in sources {
            fm.push_str(&format!("  - {s}\n"));
        }
    }
    fm.push_str("---\n\nBody.\n");
    fm
}

fn page_with_concepts(title: &str, concepts: &[&str]) -> String {
    let mut fm = format!(
        "---\ntitle: \"{title}\"\nsummary: \"s\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: concept\n"
    );
    if !concepts.is_empty() {
        fm.push_str("concepts:\n");
        for c in concepts {
            fm.push_str(&format!("  - {c}\n"));
        }
    }
    fm.push_str("---\n\nBody.\n");
    fm
}

fn page_with_body_links(title: &str, body: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"s\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: concept\n---\n\n{body}\n"
    )
}

fn simple_page(title: &str, page_type: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"s\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: {page_type}\n---\n\nBody.\n"
    )
}

fn default_filter() -> GraphFilter {
    GraphFilter {
        root: None,
        depth: None,
        types: Vec::new(),
    }
}

// ── build_graph ───────────────────────────────────────────────────────────────

#[test]
fn build_graph_creates_edges_from_sources_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_sources("MoE", &["sources/switch"]),
    );
    write_page(
        &wiki_root,
        "sources/switch.md",
        &simple_page("Switch", "paper"),
    );

    let g = build_graph(&wiki_root, &default_filter());
    assert_eq!(g.node_count(), 2);
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn build_graph_creates_edges_from_concepts_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_concepts("MoE", &["concepts/scaling"]),
    );
    write_page(
        &wiki_root,
        "concepts/scaling.md",
        &simple_page("Scaling", "concept"),
    );

    let g = build_graph(&wiki_root, &default_filter());
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn build_graph_creates_edges_from_body_wikilinks() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_body_links("MoE", "See [[concepts/scaling]] for details."),
    );
    write_page(
        &wiki_root,
        "concepts/scaling.md",
        &simple_page("Scaling", "concept"),
    );

    let g = build_graph(&wiki_root, &default_filter());
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn build_graph_skips_broken_references() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_sources("MoE", &["sources/nonexistent"]),
    );

    let g = build_graph(&wiki_root, &default_filter());
    assert_eq!(g.node_count(), 1);
    assert_eq!(g.edge_count(), 0, "broken reference should be skipped");
}

// ── in_degree ─────────────────────────────────────────────────────────────────

#[test]
fn in_degree_returns_0_for_orphan_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/orphan.md",
        &simple_page("Orphan", "concept"),
    );

    let g = build_graph(&wiki_root, &default_filter());
    assert_eq!(in_degree(&g, "concepts/orphan"), 0);
}

#[test]
fn in_degree_returns_correct_count_for_linked_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_concepts("A", &["concepts/target"]),
    );
    write_page(
        &wiki_root,
        "concepts/b.md",
        &page_with_concepts("B", &["concepts/target"]),
    );
    write_page(
        &wiki_root,
        "concepts/target.md",
        &simple_page("Target", "concept"),
    );

    let g = build_graph(&wiki_root, &default_filter());
    assert_eq!(in_degree(&g, "concepts/target"), 2);
}

// ── render_mermaid ────────────────────────────────────────────────────────────

#[test]
fn render_mermaid_produces_valid_mermaid_graph_td_block() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_concepts("A", &["concepts/b"]),
    );
    write_page(&wiki_root, "concepts/b.md", &simple_page("B", "concept"));

    let g = build_graph(&wiki_root, &default_filter());
    let output = render_mermaid(&g);

    assert!(output.starts_with("graph TD\n"));
    assert!(output.contains("concepts/a --> concepts/b"));
}

// ── render_dot ────────────────────────────────────────────────────────────────

#[test]
fn render_dot_produces_valid_dot_digraph_block() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_concepts("A", &["concepts/b"]),
    );
    write_page(&wiki_root, "concepts/b.md", &simple_page("B", "concept"));

    let g = build_graph(&wiki_root, &default_filter());
    let output = render_dot(&g);

    assert!(output.starts_with("digraph wiki {\n"));
    assert!(output.contains("\"concepts/a\" -> \"concepts/b\""));
    assert!(output.ends_with("}\n"));
}

// ── subgraph ──────────────────────────────────────────────────────────────────

#[test]
fn subgraph_returns_only_nodes_within_depth_hops_of_root() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // Chain: a -> b -> c -> d
    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_concepts("A", &["concepts/b"]),
    );
    write_page(
        &wiki_root,
        "concepts/b.md",
        &page_with_concepts("B", &["concepts/c"]),
    );
    write_page(
        &wiki_root,
        "concepts/c.md",
        &page_with_concepts("C", &["concepts/d"]),
    );
    write_page(&wiki_root, "concepts/d.md", &simple_page("D", "concept"));

    let full = build_graph(&wiki_root, &default_filter());
    let sub = subgraph(&full, "concepts/a", 2);

    // depth 2 from a: a(0) -> b(1) -> c(2), d should be excluded
    let slugs: Vec<String> = sub.node_indices().map(|i| sub[i].slug.clone()).collect();
    assert!(slugs.contains(&"concepts/a".to_string()));
    assert!(slugs.contains(&"concepts/b".to_string()));
    assert!(slugs.contains(&"concepts/c".to_string()));
    assert!(
        !slugs.contains(&"concepts/d".to_string()),
        "d is 3 hops away, should be excluded: {slugs:?}"
    );
}

#[test]
fn subgraph_with_depth_0_returns_only_root_node() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_concepts("A", &["concepts/b"]),
    );
    write_page(&wiki_root, "concepts/b.md", &simple_page("B", "concept"));

    let full = build_graph(&wiki_root, &default_filter());
    let sub = subgraph(&full, "concepts/a", 0);

    assert_eq!(sub.node_count(), 1);
    assert_eq!(sub[sub.node_indices().next().unwrap()].slug, "concepts/a");
}


// ── wrap_graph_md ─────────────────────────────────────────────────────────────

#[test]
fn wrap_graph_md_prepends_frontmatter_with_status_generated() {
    let filter = GraphFilter {
        root: Some("concepts/moe".into()),
        depth: Some(3),
        types: vec!["concept".into(), "paper".into()],
    };
    let rendered = "graph TD\n  concepts/moe --> sources/switch\n";
    let output = wrap_graph_md(rendered, "mermaid", &filter);

    assert!(output.starts_with("---\n"));
    assert!(output.contains("title: \"Wiki Graph\""));
    assert!(output.contains("status: generated"));
    assert!(output.contains("format: mermaid"));
    assert!(output.contains("root: concepts/moe"));
    assert!(output.contains("depth: 3"));
    assert!(output.contains("types: [concept, paper]"));
    assert!(output.contains("generated:"));
    assert!(output.contains("```mermaid\n"));
    assert!(output.contains("concepts/moe --> sources/switch"));
    assert!(output.ends_with("```\n"));
}

#[test]
fn wrap_graph_md_handles_empty_filter() {
    let filter = GraphFilter::default();
    let rendered = "graph TD\n";
    let output = wrap_graph_md(rendered, "mermaid", &filter);

    assert!(output.contains("root: \n"));
    assert!(output.contains("depth: 0"));
    assert!(output.contains("types: []"));
}

// ── graph output auto-commit ──────────────────────────────────────────────────

#[test]
fn graph_output_md_inside_wiki_root_commits() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_concepts("A", &["concepts/b"]),
    );
    write_page(&wiki_root, "concepts/b.md", &simple_page("B", "concept"));

    let head_before = git::current_head(dir.path()).unwrap();

    let filter = default_filter();
    let g = build_graph(&wiki_root, &filter);
    let rendered = render_mermaid(&g);
    let content = wrap_graph_md(&rendered, "mermaid", &filter);

    let out_path = wiki_root.join("graph.md");
    fs::write(&out_path, &content).unwrap();
    git::commit(dir.path(), "graph: test").unwrap();

    let head_after = git::current_head(dir.path()).unwrap();
    assert_ne!(head_before, head_after);

    // Verify the file has frontmatter
    let written = fs::read_to_string(&out_path).unwrap();
    assert!(written.contains("status: generated"));
}
