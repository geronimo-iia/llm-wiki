import json


def _rebuild(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")


def test_lint_all_rules(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", check=False)
    combined = result.stdout + result.stderr
    assert "error" in combined.lower() or "warning" in combined.lower()


def test_lint_broken_link_rule(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "broken-link", check=False)
    assert "broken-link" in result.stdout


def test_lint_orphan_rule(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "orphan", check=False)
    assert "orphan" in result.stdout


def test_lint_json_has_findings_array(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--format", "json", check=False)
    data = json.loads(result.stdout)
    assert isinstance(data.get("findings"), list)


def test_lint_broken_link_finds_dead_ref(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "broken-link", "--format", "json", check=False)
    data = json.loads(result.stdout)
    bl = [f for f in data["findings"] if f["rule"] == "broken-link"]
    assert len(bl) > 0


def test_lint_broken_link_detects_commonmark_inline(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "broken-link", "--format", "json", check=False)
    data = json.loads(result.stdout)
    msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert any("also-does-not-exist" in m for m in msgs)


def test_lint_broken_link_ignores_valid_link(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "broken-link", "--format", "json", check=False)
    data = json.loads(result.stdout)
    msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert not any("mixture-of-experts" in m for m in msgs)


def test_lint_orphan_finds_orphan_concept(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "orphan", "--format", "json", check=False)
    data = json.loads(result.stdout)
    slugs = [f["slug"] for f in data["findings"]]
    assert "concepts/orphan-concept" in slugs


def test_lint_broken_link_detects_relative_commonmark(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "broken-link", "--format", "json", check=False)
    data = json.loads(result.stdout)
    msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert any("relative-nonexistent" in m for m in msgs), (
        "relative ./relative-nonexistent.md should be flagged as broken after normalization"
    )


def test_lint_broken_link_ignores_valid_relative_link(wiki_env):
    _rebuild(wiki_env)
    result = wiki_env.run("lint", "--rules", "broken-link", "--format", "json", check=False)
    data = json.loads(result.stdout)
    msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert not any("sparse-routing" in m for m in msgs), (
        "relative ./sparse-routing.md resolves to an existing page and must not be flagged"
    )


def test_lint_structural_rules_run(wiki_env):
    _rebuild(wiki_env)
    for rule in ("articulation-point", "bridge", "periphery"):
        result = wiki_env.run("lint", "--rules", rule, check=False)
        assert result.returncode in (0, 1)
