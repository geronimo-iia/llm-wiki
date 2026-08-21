from conftest import SPACE_NAME


async def test_stats_returns_wiki_name(mcp_env):
    await mcp_env.rebuild()
    text = await mcp_env.call("wiki_stats")
    assert isinstance(text, str)
    assert SPACE_NAME in text


async def test_stats_json_required_keys(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"format": "json"})
    assert isinstance(data, dict)
    for key in ("pages", "orphans"):
        assert key in data, f"missing key: {key}"
        assert isinstance(data[key], int), f"{key} should be int, got {type(data[key])}"
        assert data[key] >= 0, f"{key} should be >= 0"


async def test_stats_json_pages_gt_0(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"format": "json"})
    assert data["pages"] > 0


async def test_stats_json_orphans_gte_0(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"format": "json"})
    assert data["orphans"] >= 0


async def test_stats_communities_present(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"format": "json"})
    assert "communities" in data
    comm = data["communities"]
    assert isinstance(comm, dict)
    assert isinstance(comm["count"], int)
    assert comm["count"] >= 0


async def test_stats_diameter_field(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"format": "json"})
    assert "diameter" in data
    assert data["diameter"] is None or isinstance(data["diameter"], (int, float))


async def test_stats_center_is_array(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"format": "json"})
    assert "center" in data
    assert isinstance(data["center"], list)
    for slug in data["center"]:
        assert isinstance(slug, str)


async def test_stats_staleness_shape(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"format": "json"})
    assert "staleness" in data, f"expected 'staleness' key in stats output, got keys: {list(data)}"
    s = data["staleness"]
    for bucket in ("fresh", "stale_7d", "stale_30d"):
        assert bucket in s, f"expected '{bucket}' in staleness, got: {list(s)}"
        assert isinstance(s[bucket], int), f"staleness.{bucket} should be int, got {type(s[bucket])}"
        assert s[bucket] >= 0, f"staleness.{bucket} should be >= 0"
    total = s["fresh"] + s["stale_7d"] + s["stale_30d"]
    assert total == data["pages"], (
        f"staleness buckets must sum to page count: {total} != {data['pages']}"
    )
