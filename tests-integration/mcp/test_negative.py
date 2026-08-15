from conftest import SLUG_MISSING, SPACE_NAME


async def test_read_missing_page_returns_error(mcp_env):
    await mcp_env.rebuild()
    is_error, text = await mcp_env.call_raw(
        "wiki_content_read", {"uri": SLUG_MISSING, "wiki": SPACE_NAME}
    )
    assert is_error is True
    assert len(text) > 0


async def test_search_empty_query_returns_valid_response(mcp_env):
    await mcp_env.rebuild()
    is_error, text = await mcp_env.call_raw(
        "wiki_search", {"query": "", "wiki": SPACE_NAME, "format": "json"}
    )
    if is_error:
        assert len(text) > 0
    else:
        import json

        data = json.loads(text)
        assert isinstance(data, dict)
        assert "results" in data
        assert isinstance(data["results"], list)


async def test_lint_missing_page_does_not_crash(mcp_env):
    await mcp_env.rebuild()
    is_error, text = await mcp_env.call_raw("wiki_lint", {"uri": SLUG_MISSING, "wiki": SPACE_NAME})
    assert len(text) > 0


async def test_graph_invalid_format_returns_error(mcp_env):
    await mcp_env.rebuild()
    is_error, text = await mcp_env.call_raw(
        "wiki_graph", {"format": "invalid-format-xyz", "wiki": SPACE_NAME}
    )
    assert is_error is True
    assert "invalid-format-xyz" in text or "unknown" in text.lower()


async def test_resolve_missing_page_not_crash(mcp_env):
    data = await mcp_env.json("wiki_resolve", {"uri": SLUG_MISSING, "wiki": SPACE_NAME})
    assert isinstance(data, dict)
    assert data["exists"] is False
    assert isinstance(data["path"], str)
