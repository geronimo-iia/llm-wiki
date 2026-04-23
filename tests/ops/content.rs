use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;

// ── Content ───────────────────────────────────────────────────────────────────

#[test]
fn content_read_page() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    match ops::content_read(&engine, "concepts/moe", None, false, false).unwrap() {
        ops::ContentReadResult::Page(content) => {
            assert!(content.contains("Mixture of Experts"));
        }
        _ => panic!("expected Page"),
    }
}

#[test]
fn content_read_no_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    match ops::content_read(&engine, "concepts/moe", None, true, false).unwrap() {
        ops::ContentReadResult::Page(content) => {
            assert!(!content.contains("title:"));
            assert!(content.contains("Mixture of Experts"));
        }
        _ => panic!("expected Page"),
    }
}

#[test]
fn content_write_and_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let body = "---\ntitle: \"New\"\ntype: page\n---\n\nHello.\n";
    let result = ops::content_write(&engine, "new-page", None, body).unwrap();
    assert_eq!(result.bytes_written, body.len());

    match ops::content_read(&engine, "new-page", None, false, false).unwrap() {
        ops::ContentReadResult::Page(content) => assert!(content.contains("Hello.")),
        _ => panic!("expected Page"),
    }
}

#[test]
fn content_new_page() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let uri = ops::content_new(
        &engine,
        "concepts/new-concept",
        None,
        false,
        false,
        None,
        None,
    )
    .unwrap();
    assert!(uri.starts_with("wiki://test/concepts/new-concept"));
}

#[test]
fn content_new_section() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let uri = ops::content_new(&engine, "topics", None, true, false, None, None).unwrap();
    assert!(uri.contains("topics"));
}

#[test]
fn content_commit_all() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    // Write a new file so there's something to commit
    ops::content_write(
        &engine,
        "scratch",
        None,
        "---\ntitle: \"Scratch\"\ntype: page\n---\n\ntemp\n",
    )
    .unwrap();

    let hash = ops::content_commit(&engine, "test", &[], true, Some("test commit")).unwrap();
    assert!(!hash.is_empty());
}
