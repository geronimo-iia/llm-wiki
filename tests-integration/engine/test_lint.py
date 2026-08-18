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


def test_lint_cross_wiki_body_link_no_false_positive(wiki_env):
    # Write a page with a cross-wiki body link pointing to the mounted "notes" wiki.
    # The broken-link rule must not fire — wiki:// links to mounted wikis are skipped
    # at src/ops/lint.rs:335 (continue branch). broken-cross-wiki-link fires only when
    # the target wiki is NOT mounted.
    page = wiki_env.research_wiki / "concepts" / "cross-ref.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    page.write_text(
        "---\ntitle: Cross Ref\ntype: concept\n---\n\n"
        "See [attention mechanism](wiki://notes/concepts/attention-mechanism).\n"
    )
    wiki_env.run("index", "rebuild", "--wiki", "research")
    result = wiki_env.run("lint", "--rules", "broken-link", "--format", "json", check=False)
    data = json.loads(result.stdout)
    bl_msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert not any("wiki://notes" in m for m in bl_msgs), (
        "cross-wiki body link to a mounted wiki must not trigger broken-link"
    )
