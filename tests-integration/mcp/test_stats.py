from conftest import SPACE_NAME


async def test_stats_returns_wiki_name(mcp_env):
    await mcp_env.rebuild()
    text = await mcp_env.call("wiki_stats")
    assert isinstance(text, str)
    assert SPACE_NAME in text


async def test_stats_json_required_keys(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"detail": "full"})
    assert isinstance(data, dict)
    for key in ("pages", "orphans"):
        assert key in data, f"missing key: {key}"
        assert isinstance(data[key], int), f"{key} should be int, got {type(data[key])}"
        assert data[key] >= 0, f"{key} should be >= 0"


async def test_stats_json_pages_gt_0(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"detail": "full"})
    assert data["pages"] > 0


async def test_stats_json_orphans_gte_0(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"detail": "full"})
    assert data["orphans"] >= 0


async def test_stats_communities_present(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"detail": "full"})
    assert "communities" in data
    comm = data["communities"]
    assert isinstance(comm, dict)
    assert isinstance(comm["count"], int)
    assert comm["count"] >= 0


async def test_stats_diameter_field(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"detail": "full"})
    assert "diameter" in data
    assert data["diameter"] is None or isinstance(data["diameter"], (int, float))


async def test_stats_center_count_in_summary(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats")
    assert "center_count" in data, "summary mode must include center_count"
    assert isinstance(data["center_count"], int)
    assert data["center_count"] >= 0


async def test_stats_staleness_shape(mcp_env):
    await mcp_env.rebuild()
    data = await mcp_env.json("wiki_stats", {"detail": "full"})
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
