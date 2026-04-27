use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::index_manager::SpaceIndexManager;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::ops::backlinks_query;
use llm_wiki::space_builder;
use llm_wiki::type_registry::SpaceTypeRegistry;

fn schema() -> IndexSchema {
    let (_registry, schema) = space_builder::build_space_from_embedded("en_stem");
    schema
}

fn registry() -> SpaceTypeRegistry {
    let (registry, _schema) = space_builder::build_space_from_embedded("en_stem");
    registry
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

fn build_index(dir: &Path, wiki_root: &Path) -> SpaceIndexManager {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    let mgr = SpaceIndexManager::new("test", &index_path);
    mgr.rebuild(wiki_root, dir, &schema(), &registry()).unwrap();
    mgr.open(&schema(), None).unwrap();
    mgr
}

#[test]
fn backlinks_for_returns_pages_that_link_to_target() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // target page
    write_page(
        &wiki_root,
        "concepts/target.md",
        "---\ntitle: \"Target\"\ntype: concept\nread_when: [\"x\"]\n---\n\nThe target page.\n",
    );
    // two pages linking to it via [[concepts/target]]
    write_page(
        &wiki_root,
        "concepts/alpha.md",
        "---\ntitle: \"Alpha\"\ntype: concept\nread_when: [\"x\"]\n---\n\nSee [[concepts/target]] for details.\n",
    );
    write_page(
        &wiki_root,
        "concepts/beta.md",
        "---\ntitle: \"Beta\"\ntype: concept\nread_when: [\"x\"]\n---\n\nAlso links [[concepts/target]] here.\n",
    );
    // unrelated page — no link to target
    write_page(
        &wiki_root,
        "concepts/unrelated.md",
        "---\ntitle: \"Unrelated\"\ntype: concept\nread_when: [\"x\"]\n---\n\nNo links here.\n",
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let searcher = mgr.searcher().unwrap();
    let is = schema();

    let refs = backlinks_query(&searcher, &is, "concepts/target").unwrap();
    let slugs: Vec<&str> = refs.iter().map(|r| r.slug.as_str()).collect();

    assert_eq!(refs.len(), 2, "expected 2 backlinks, got: {slugs:?}");
    assert!(
        slugs.contains(&"concepts/alpha"),
        "alpha should be in backlinks"
    );
    assert!(
        slugs.contains(&"concepts/beta"),
        "beta should be in backlinks"
    );
    assert!(
        !slugs.contains(&"concepts/unrelated"),
        "unrelated should not be in backlinks"
    );
}

#[test]
fn backlinks_for_returns_empty_when_no_incoming_links() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    write_page(
        &wiki_root,
        "concepts/isolated.md",
        "---\ntitle: \"Isolated\"\ntype: concept\nread_when: [\"x\"]\n---\n\nNo one links here.\n",
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let searcher = mgr.searcher().unwrap();
    let is = schema();

    let refs = backlinks_query(&searcher, &is, "concepts/isolated").unwrap();
    assert!(refs.is_empty(), "expected no backlinks for isolated page");
}
