use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;

#[test]
fn stats_returns_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::stats(&engine, "test").unwrap();
    assert_eq!(result.wiki, "test");
    assert!(result.pages >= 2);
    assert!(result.types.contains_key("concept"));
}

#[test]
fn stats_orphan_count() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::stats(&engine, "test").unwrap();
    // Both pages are concepts with no inbound edges from other types
    // (only a body wikilink from transformer to moe)
    assert!(
        result.orphans <= result.pages,
        "orphans should not exceed total pages"
    );
}

#[test]
fn stats_staleness_buckets_sum_to_total() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::stats(&engine, "test").unwrap();
    let staleness_total =
        result.staleness.fresh + result.staleness.stale_7d + result.staleness.stale_30d;
    assert_eq!(
        staleness_total, result.pages,
        "staleness buckets should sum to total pages"
    );
}

#[test]
fn stats_on_empty_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("empty");
    llm_wiki::spaces::create(&wiki_path, "empty", None, false, true, &config_path, None).unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::stats(&engine, "empty").unwrap();
    assert_eq!(result.pages, 0);
    assert_eq!(result.orphans, 0);
    assert_eq!(result.staleness.fresh, 0);
}
