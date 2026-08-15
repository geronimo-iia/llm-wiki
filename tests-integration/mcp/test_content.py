from pathlib import Path

from conftest import SLUG_MISSING, SLUG_SCALING_LAWS, SPACE_NAME, SLUG_MoE


async def test_content_read_returns_page_body(mcp_env):
    text = await mcp_env.call("wiki_content_read", {"uri": SLUG_MoE})
    assert isinstance(text, str)
    assert len(text) > 0
    assert "---" in text
    assert "Mixture of Experts" in text


async def test_content_read_includes_frontmatter(mcp_env):
    text = await mcp_env.call("wiki_content_read", {"uri": SLUG_MoE})
    assert isinstance(text, str)
    assert text.startswith("---")
    assert "type:" in text


async def test_content_read_with_backlinks(mcp_env):
    text = await mcp_env.call("wiki_content_read", {"uri": SLUG_MoE, "backlinks": True})
    assert isinstance(text, str)
    assert "---" in text
    assert "backlinks" in text


async def test_content_read_backlinks_include_scaling_laws(mcp_env):
    text = await mcp_env.call("wiki_content_read", {"uri": SLUG_SCALING_LAWS, "backlinks": True})
    assert "mixture-of-experts" in text


async def test_content_read_via_wiki_uri(mcp_env):
    text = await mcp_env.call("wiki_content_read", {"uri": f"wiki://{SPACE_NAME}/{SLUG_MoE}"})
    assert "Mixture of Experts" in text


async def test_resolve_existing_slug_exists_true(mcp_env):
    data = await mcp_env.json("wiki_resolve", {"uri": SLUG_MoE})
    assert isinstance(data, dict)
    assert data["exists"] is True
    assert isinstance(data["slug"], str)
    assert isinstance(data["path"], str)
    assert data["path"].endswith(".md")


async def test_resolve_existing_slug_has_md_path(mcp_env):
    data = await mcp_env.json("wiki_resolve", {"uri": SLUG_MoE})
    assert data["path"].endswith(".md")


async def test_resolve_nonexistent_slug_exists_false(mcp_env):
    data = await mcp_env.json("wiki_resolve", {"uri": SLUG_MISSING})
    assert isinstance(data, dict)
    assert data["exists"] is False
    assert isinstance(data["path"], str)


async def test_resolve_nonexistent_slug_returns_would_be_path(mcp_env):
    data = await mcp_env.json("wiki_resolve", {"uri": SLUG_MISSING})
    assert data["path"].endswith(".md")


async def test_resolve_returns_slug(mcp_env):
    data = await mcp_env.json("wiki_resolve", {"uri": SLUG_MoE})
    assert isinstance(data["slug"], str)
    assert len(data["slug"]) > 0


async def test_content_write_and_read_back(mutable_mcp_env):
    new_data = await mutable_mcp_env.json(
        "wiki_content_new",
        {"uri": "concepts/test-write-target", "wiki": SPACE_NAME},
    )
    slug = new_data["slug"]

    content = "---\ntitle: Write Test\ntype: page\nstatus: draft\n---\n\nHello write test.\n"
    await mutable_mcp_env.call(
        "wiki_content_write",
        {"uri": slug, "content": content, "wiki": SPACE_NAME},
    )

    text = await mutable_mcp_env.call("wiki_content_read", {"uri": slug, "wiki": SPACE_NAME})
    assert "Hello write test." in text


async def test_content_new_creates_page(mutable_mcp_env):
    data = await mutable_mcp_env.json(
        "wiki_content_new",
        {"uri": "concepts/test-new-page", "wiki": SPACE_NAME},
    )
    assert data["slug"] == "concepts/test-new-page"
    assert data["path"].endswith(".md")
    assert Path(data["path"]).exists()


async def test_content_commit_after_write(mutable_mcp_env):
    new_data = await mutable_mcp_env.json(
        "wiki_content_new",
        {"uri": "concepts/test-commit-target", "wiki": SPACE_NAME},
    )
    slug = new_data["slug"]
    content = "---\ntitle: Commit Test\ntype: page\nstatus: draft\n---\n\nCommit me.\n"
    await mutable_mcp_env.call(
        "wiki_content_write",
        {"uri": slug, "content": content, "wiki": SPACE_NAME},
    )

    result_text = await mutable_mcp_env.call(
        "wiki_content_commit",
        {"slugs": [slug], "message": "test: commit test page", "wiki": SPACE_NAME},
    )
    assert result_text, "commit response should not be empty"
    assert len(result_text) > 5, "commit hash should be a valid git SHA"

    resolved = await mutable_mcp_env.json("wiki_resolve", {"uri": slug, "wiki": SPACE_NAME})
    assert resolved["exists"] is True, f"page {slug} should exist after commit"
