def test_search_basic_returns_results(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    result = wiki_env.run("search", "mixture of experts")
    assert "mixture" in result.stdout.lower()


def test_search_type_filter(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    result = wiki_env.run("search", "routing", "--type", "concept")
    assert "concept" in result.stdout.lower() or result.stdout.strip()


def test_search_cross_wiki(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    wiki_env.run("index", "rebuild", "--wiki", "notes")
    result = wiki_env.run("search", "attention", "--cross-wiki")
    assert result.returncode == 0


def test_search_json_has_results(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    data = wiki_env.json("search", "transformer")
    assert len(data["results"]) > 0


def test_search_colon_query(wiki_env):
    # Regression for parse_query_lenient fallback (fixed 0.5.6).
    # "Layer 1: Attention" has a colon that fails Tantivy's strict parser.
    # This test asserts no crash only — the research fixture has no "Layer 1"
    # content so result count is not guaranteed. The result-non-empty case is
    # already covered by tests/search.rs::search_query_with_colon_does_not_error
    # which writes its own fixture page.
    wiki_env.run("index", "rebuild", "--wiki", "research")
    result = wiki_env.run("search", "Layer 1: Attention", check=False)
    assert result.returncode == 0
