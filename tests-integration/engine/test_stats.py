def test_stats_returns_output(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    result = wiki_env.run("stats")
    assert "research" in result.stdout


def test_stats_json_pages(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    data = wiki_env.json("stats")
    assert data["pages"] > 0


def test_stats_json_fields(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    data = wiki_env.json("stats")
    assert "communities" in data
    assert "diameter" in data
    assert "radius" in data
    assert "center_count" in data, "summary mode must include center_count"
    assert isinstance(data["center_count"], int)
    assert data["center_count"] >= 0


def test_stats_staleness_shape(wiki_env):
    wiki_env.run("index", "rebuild", "--wiki", "research")
    data = wiki_env.json("stats")
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
