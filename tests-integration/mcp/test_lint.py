import pytest

from conftest import SLUG_ORPHAN, SPACE_NAME


@pytest.mark.parametrize("rule", ["broken-link", "orphan"])
async def test_lint_rule_appears_in_text(mcp_env, rule):
    await mcp_env.rebuild()
    text = await mcp_env.call("wiki_lint", {"rules": [rule]})
    assert rule in text


@pytest.mark.parametrize("rule", ["broken-link", "orphan"])
async def test_lint_rule_has_findings(mcp_env, rule):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_lint", {"rules": [rule], "format": "json"})
    assert isinstance(data["findings"], list)
    assert len(data["findings"]) > 0
    matching = [f for f in data["findings"] if f["rule"] == rule]
    assert len(matching) > 0, f"no findings with rule={rule!r}"
    for f in matching:
        assert isinstance(f.get("slug"), str)


async def test_lint_returns_findings(mcp_env):
    await mcp_env.rebuild()
    text = await mcp_env.call("wiki_lint")
    combined = text.lower()
    assert "error" in combined or "warning" in combined


async def test_lint_json_findings_array(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_lint", {"format": "json"})
    assert isinstance(data["findings"], list)
    assert len(data["findings"]) > 0


async def test_lint_broken_link_detects_also_does_not_exist(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_lint", {"rules": ["broken-link"], "format": "json"})
    msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert any("also-does-not-exist" in m for m in msgs)


async def test_lint_orphan_finds_orphan_concept(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_lint", {"rules": ["orphan"], "format": "json"})
    slugs = [f["slug"] for f in data["findings"]]
    assert SLUG_ORPHAN in slugs


async def test_lint_with_wiki_param(mcp_env):
    await mcp_env.rebuild()
    text = await mcp_env.call("wiki_lint", {"wiki": SPACE_NAME})
    combined = text.lower()
    assert "error" in combined or "warning" in combined


async def test_lint_broken_link_detects_relative_commonmark(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_lint", {"rules": ["broken-link"], "format": "json"})
    msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert any("relative-nonexistent" in m for m in msgs), (
        "relative ./relative-nonexistent.md should be flagged as broken after normalization"
    )


async def test_lint_broken_link_ignores_valid_relative_link(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_lint", {"rules": ["broken-link"], "format": "json"})
    msgs = [f["message"] for f in data["findings"] if f["rule"] == "broken-link"]
    assert not any("sparse-routing" in m for m in msgs), (
        "relative ./sparse-routing.md resolves to an existing page and must not be flagged"
    )


async def test_lint_findings_have_md_path(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_lint", {"rules": ["broken-link"], "format": "json"})
    for f in data["findings"]:
        assert f.get("path", "").endswith(".md"), f"finding path not .md: {f.get('path')}"
