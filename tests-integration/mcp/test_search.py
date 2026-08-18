from conftest import SLUG_MoE


async def test_search_returns_results(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_search", {"query": "mixture of experts", "format": "json"})
    assert isinstance(data, dict)
    assert "results" in data
    assert isinstance(data["results"], list)
    assert len(data["results"]) > 0
    hit = data["results"][0]
    assert isinstance(hit.get("slug"), str)
    assert isinstance(hit.get("title"), str)
    assert isinstance(hit.get("score"), (int, float))
    assert hit["score"] > 0
    assert SLUG_MoE in [r["slug"] for r in data["results"]]


async def test_search_json_results_not_empty(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_search", {"query": "mixture of experts", "format": "json"})
    assert len(data["results"]) > 0
    for hit in data["results"]:
        assert isinstance(hit["slug"], str)
        assert isinstance(hit["title"], str)
        assert isinstance(hit["score"], (int, float))


async def test_search_type_filter(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json(
        "wiki_search", {"query": "attention", "type": "concept", "format": "json"}
    )
    assert isinstance(data, dict)
    assert isinstance(data["results"], list)
    assert len(data["results"]) > 0
    for hit in data["results"]:
        assert isinstance(hit["slug"], str)
        assert hit["slug"].startswith("concepts/")


async def test_search_llms_format(mcp_env):
    await mcp_env.rebuild()
    text = await mcp_env.call("wiki_search", {"query": "transformer", "format": "llms"})
    assert isinstance(text, str)
    assert len(text) > 0
    assert "wiki://" in text


async def test_list_json_total_gt_0(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_list", {"format": "json"})
    assert isinstance(data["total"], int)
    assert data["total"] > 0


async def test_list_json_pages_is_array(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_list", {"format": "json"})
    assert isinstance(data["pages"], list)
    assert len(data["pages"]) == data["total"]
    for page in data["pages"]:
        assert isinstance(page.get("slug"), str)
        assert isinstance(page.get("title"), str)


async def test_list_type_filter_returns_concept(mcp_env):
    await mcp_env.rebuild()
    text = await mcp_env.call("wiki_list", {"type": "concept"})
    assert isinstance(text, str)
    assert "concept" in text


async def test_list_json_type_filter_all_concepts(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_list", {"type": "concept", "format": "json"})
    assert isinstance(data["pages"], list)
    assert len(data["pages"]) > 0
    assert all(p["type"] == "concept" for p in data["pages"])


async def test_search_colon_query_lenient_fallback(mcp_env):
    """Regression: parse_query_lenient fallback must not error on field specifiers."""
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_search", {"query": "title:attention", "format": "json"})
    assert isinstance(data, dict), "expected dict response, got error"
    assert "results" in data, f"expected 'results' key, got: {data}"
    assert isinstance(data["results"], list)
