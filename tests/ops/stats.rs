use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;
use std::fs;

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

fn setup_wiki_with_cycle(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join(name);

    llm_wiki::spaces::create(&wiki_path, name, None, false, true, &config_path, None).unwrap();

    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    // Three pages forming a directed cycle: a→b→c→a
    // This guarantees a strongly connected graph where diameter/radius are defined
    fs::write(
        wiki_root.join("concepts/alpha.md"),
        "---\ntitle: \"Alpha\"\ntype: concept\nstatus: active\n---\n\nSee [[concepts/beta]].\n",
    )
    .unwrap();
    fs::write(
        wiki_root.join("concepts/beta.md"),
        "---\ntitle: \"Beta\"\ntype: concept\nstatus: active\n---\n\nSee [[concepts/gamma]].\n",
    )
    .unwrap();
    fs::write(
        wiki_root.join("concepts/gamma.md"),
        "---\ntitle: \"Gamma\"\ntype: concept\nstatus: active\n---\n\nSee [[concepts/alpha]].\n",
    )
    .unwrap();
    llm_wiki::git::commit(&wiki_path, "add cycle pages").unwrap();

    config_path
}

#[test]
fn stats_structural_fields_present_on_connected_graph() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki_with_cycle(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::stats(&engine, "test").unwrap();
    // Structural fields must always be present as keys (Some or None)
    // On a small connected graph with structural_algorithms=true (default),
    // diameter and radius must be Some
    assert!(
        result.diameter.is_some(),
        "diameter should be Some on a small strongly-connected graph, got None"
    );
    assert!(
        result.radius.is_some(),
        "radius should be Some on a small strongly-connected graph, got None"
    );
    assert!(
        !result.center.is_empty(),
        "center should be non-empty on a connected graph"
    );
    assert!(
        result.structural_note.is_none(),
        "structural_note should be None when algorithms ran, got: {:?}",
        result.structural_note
    );
    assert!(
        result.diameter.unwrap() >= result.radius.unwrap(),
        "diameter must be >= radius"
    );
}

#[test]
fn stats_structural_fields_null_when_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki_with_cycle(dir.path(), "test");

    // Disable structural_algorithms via config before building engine.
    // The generated config already contains a [graph] section, so we replace the
    // structural_algorithms line rather than appending a duplicate section header.
    let config_str = std::fs::read_to_string(&config_path).unwrap();
    let patched = if config_str.contains("structural_algorithms") {
        config_str.replace("structural_algorithms = true", "structural_algorithms = false")
    } else {
        // Insert the key inside the existing [graph] section
        config_str.replace("[graph]", "[graph]\nstructural_algorithms = false")
    };
    std::fs::write(&config_path, patched).unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::stats(&engine, "test").unwrap();
    assert!(
        result.diameter.is_none(),
        "diameter must be None when structural_algorithms=false"
    );
    assert!(
        result.radius.is_none(),
        "radius must be None when structural_algorithms=false"
    );
    assert!(
        result.center.is_empty(),
        "center must be empty when structural_algorithms=false"
    );
    assert!(
        result.structural_note.is_none(),
        "structural_note must be None when disabled (not skipped-due-to-size)"
    );
}
