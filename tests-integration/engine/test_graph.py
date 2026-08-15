def _rebuild_both(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    wiki_env.run("index", "rebuild", "--wiki", "notes")


def test_graph_mermaid(wiki_env):
    _rebuild_both(wiki_env)
    result = wiki_env.run("graph")
    assert "graph" in result.stdout.lower()


def test_graph_dot(wiki_env):
    _rebuild_both(wiki_env)
    result = wiki_env.run("graph", "--format", "dot")
    assert "digraph" in result.stdout


def test_graph_llms(wiki_env):
    _rebuild_both(wiki_env)
    result = wiki_env.run("graph", "--format", "llms")
    assert result.returncode == 0
    assert len(result.stdout.strip()) > 0


def test_graph_type_filter(wiki_env):
    _rebuild_both(wiki_env)
    result = wiki_env.run("graph", "--type", "concept")
    assert result.returncode == 0


def test_graph_root_depth(wiki_env):
    _rebuild_both(wiki_env)
    result = wiki_env.run(
        "graph",
        "--root",
        "concepts/mixture-of-experts",
        "--depth",
        "2",
    )
    assert result.returncode == 0


def test_graph_cross_wiki(wiki_env):
    _rebuild_both(wiki_env)
    result = wiki_env.run("graph", "--cross-wiki")
    assert result.returncode == 0
    assert len(result.stdout.strip()) > 0
