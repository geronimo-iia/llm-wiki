use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::graph::*;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::indexing;
use llm_wiki::search;
use llm_wiki::type_registry::SpaceTypeRegistry;

fn schema() -> IndexSchema {
    IndexSchema::build("en_stem")
}

fn registry() -> SpaceTypeRegistry {
    SpaceTypeRegistry::from_embedded()
}

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

fn build_index(dir: &Path, wiki_root: &Path) -> std::path::PathBuf {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    indexing::rebuild_index(wiki_root, &index_path, "test", dir, &schema(), &registry()).unwrap();
    index_path
}

fn page_with_body_links(title: &str, body: &str) -> String {
    format!("---\ntitle: \"{title}\"\ntype: concept\nstatus: active\n---\n\n{body}\n")
}

fn simple_page(title: &str, page_type: &str) -> String {
    format!("---\ntitle: \"{title}\"\ntype: {page_type}\nstatus: active\n---\n\nBody.\n")
}

fn default_filter() -> GraphFilter {
    GraphFilter::default()
}

// ── build_graph from index ────────────────────────────────────────────────────

#[test]
fn build_graph_creates_nodes_from_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &simple_page("MoE", "concept"),
    );
    write_page(
        &wiki_root,
        "sources/switch.md",
        &simple_page("Switch", "paper"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let g = build_graph(&index_path, &is, &default_filter()).unwrap();

    assert_eq!(g.node_count(), 2);
}

#[test]
fn build_graph_creates_edges_from_body_links() {
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

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let g = build_graph(&index_path, &is, &default_filter()).unwrap();

    assert_eq!(g.edge_count(), 1);
    let edge = g.edge_indices().next().unwrap();
    assert_eq!(g[edge].relation, "links-to");
}

#[test]
fn build_graph_skips_broken_references() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_body_links("MoE", "See [[concepts/nonexistent]]."),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let g = build_graph(&index_path, &is, &default_filter()).unwrap();

    assert_eq!(g.node_count(), 1);
    assert_eq!(g.edge_count(), 0);
}

// ── type filter ───────────────────────────────────────────────────────────────

#[test]
fn build_graph_type_filter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &simple_page("MoE", "concept"),
    );
    write_page(
        &wiki_root,
        "sources/switch.md",
        &simple_page("Switch", "paper"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let filter = GraphFilter {
        types: vec!["concept".into()],
        ..Default::default()
    };
    let g = build_graph(&index_path, &is, &filter).unwrap();

    assert_eq!(g.node_count(), 1);
    assert_eq!(g[g.node_indices().next().unwrap()].r#type, "concept");
}

// ── relation filter ───────────────────────────────────────────────────────────

#[test]
fn build_graph_relation_filter_excludes_non_matching() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_body_links("MoE", "See [[concepts/scaling]]."),
    );
    write_page(
        &wiki_root,
        "concepts/scaling.md",
        &simple_page("Scaling", "concept"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    // Filter for "fed-by" — should exclude "links-to" edges
    let filter = GraphFilter {
        relation: Some("fed-by".into()),
        ..Default::default()
    };
    let g = build_graph(&index_path, &is, &filter).unwrap();
    assert_eq!(g.edge_count(), 0);
}

// ── render_mermaid ────────────────────────────────────────────────────────────

#[test]
fn render_mermaid_includes_titles_and_relations() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_body_links("MoE", "See [[concepts/scaling]]."),
    );
    write_page(
        &wiki_root,
        "concepts/scaling.md",
        &simple_page("Scaling", "concept"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let g = build_graph(&index_path, &is, &default_filter()).unwrap();
    let output = render_mermaid(&g);

    assert!(output.starts_with("graph LR\n"));
    assert!(output.contains("\"MoE\""));
    assert!(output.contains("\"Scaling\""));
    assert!(output.contains(":::concept"));
    assert!(output.contains("|links-to|"));
}

// ── render_dot ────────────────────────────────────────────────────────────────

#[test]
fn render_dot_includes_labels_and_relations() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &page_with_body_links("MoE", "See [[concepts/scaling]]."),
    );
    write_page(
        &wiki_root,
        "concepts/scaling.md",
        &simple_page("Scaling", "concept"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let g = build_graph(&index_path, &is, &default_filter()).unwrap();
    let output = render_dot(&g);

    assert!(output.starts_with("digraph wiki {\n"));
    assert!(output.contains("label=\"MoE\""));
    assert!(output.contains("type=\"concept\""));
    assert!(output.contains("label=\"links-to\""));
    assert!(output.ends_with("}\n"));
}

// ── subgraph ──────────────────────────────────────────────────────────────────

#[test]
fn subgraph_limits_depth() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // Chain: a -> b -> c -> d via body links
    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_body_links("A", "See [[concepts/b]]."),
    );
    write_page(
        &wiki_root,
        "concepts/b.md",
        &page_with_body_links("B", "See [[concepts/c]]."),
    );
    write_page(
        &wiki_root,
        "concepts/c.md",
        &page_with_body_links("C", "See [[concepts/d]]."),
    );
    write_page(&wiki_root, "concepts/d.md", &simple_page("D", "concept"));

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let filter = GraphFilter {
        root: Some("concepts/a".into()),
        depth: Some(2),
        ..Default::default()
    };
    let g = build_graph(&index_path, &is, &filter).unwrap();

    let slugs: Vec<String> = g.node_indices().map(|i| g[i].slug.clone()).collect();
    assert!(slugs.contains(&"concepts/a".to_string()));
    assert!(slugs.contains(&"concepts/b".to_string()));
    assert!(slugs.contains(&"concepts/c".to_string()));
    assert!(!slugs.contains(&"concepts/d".to_string()));
}

#[test]
fn subgraph_depth_0_returns_root_only() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_body_links("A", "See [[concepts/b]]."),
    );
    write_page(&wiki_root, "concepts/b.md", &simple_page("B", "concept"));

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let filter = GraphFilter {
        root: Some("concepts/a".into()),
        depth: Some(0),
        ..Default::default()
    };
    let g = build_graph(&index_path, &is, &filter).unwrap();

    assert_eq!(g.node_count(), 1);
    assert_eq!(g[g.node_indices().next().unwrap()].slug, "concepts/a");
}

// ── wrap_graph_md ─────────────────────────────────────────────────────────────

#[test]
fn wrap_graph_md_includes_frontmatter() {
    let filter = GraphFilter {
        root: Some("concepts/moe".into()),
        depth: Some(3),
        types: vec!["concept".into()],
        ..Default::default()
    };
    let rendered = "graph LR\n  a --> b\n";
    let output = wrap_graph_md(rendered, "mermaid", &filter);

    assert!(output.starts_with("---\n"));
    assert!(output.contains("status: generated"));
    assert!(output.contains("format: mermaid"));
    assert!(output.contains("root: concepts/moe"));
    assert!(output.contains("depth: 3"));
    assert!(output.contains("```mermaid\n"));
    assert!(output.ends_with("```\n"));
}

// ── in_degree ─────────────────────────────────────────────────────────────────

#[test]
fn in_degree_counts_incoming_edges() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/a.md",
        &page_with_body_links("A", "See [[concepts/target]]."),
    );
    write_page(
        &wiki_root,
        "concepts/b.md",
        &page_with_body_links("B", "See [[concepts/target]]."),
    );
    write_page(
        &wiki_root,
        "concepts/target.md",
        &simple_page("Target", "concept"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let g = build_graph(&index_path, &is, &default_filter()).unwrap();

    assert_eq!(in_degree(&g, "concepts/target"), 2);
}

#[test]
fn in_degree_zero_for_orphan() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/orphan.md",
        &simple_page("Orphan", "concept"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let g = build_graph(&index_path, &is, &default_filter()).unwrap();

    assert_eq!(in_degree(&g, "concepts/orphan"), 0);
}
