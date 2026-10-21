from conftest import SLUG_MoE, SPACE_NAME


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


# ── helpers ───────────────────────────────────────────────────────────────────


async def _add_page(env, slug: str, title: str, tags: list[str], confidence: float | None = None, status: str = "active") -> None:
    """Create, write, commit, and index-rebuild a page with given tags/confidence."""
    fm_tags = "[" + ", ".join(f'"{t}"' for t in tags) + "]"
    fm_conf = f"\nconfidence: {confidence}" if confidence is not None else ""
    content = (
        f"---\ntitle: \"{title}\"\ntype: concept\nstatus: {status}\ntags: {fm_tags}{fm_conf}\n---\n\n{title} body.\n"
    )
    await env.json("wiki_content_new", {"uri": slug, "wiki": SPACE_NAME})
    await env.call("wiki_content_write", {"uri": slug, "content": content, "wiki": SPACE_NAME})
    await env.call("wiki_content_commit", {"slugs": [slug], "message": f"test: add {slug}", "wiki": SPACE_NAME})
    await env.rebuild()


# ── wiki_list filters ─────────────────────────────────────────────────────────


async def test_list_tags_filter_and(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/tag-ab", "Tag AB", ["alpha", "beta"])
    await _add_page(mutable_mcp_env, "concepts/tag-a-only", "Tag A Only", ["alpha"])

    data = await mutable_mcp_env.json(
        "wiki_list", {"tags": ["alpha", "beta"], "tags_mode": "and", "format": "json"}
    )
    slugs = [p["slug"] for p in data["pages"]]
    assert "concepts/tag-ab" in slugs
    assert "concepts/tag-a-only" not in slugs


async def test_list_tags_filter_or(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/or-alpha", "Or Alpha", ["alpha"])
    await _add_page(mutable_mcp_env, "concepts/or-beta", "Or Beta", ["beta"])
    await _add_page(mutable_mcp_env, "concepts/or-neither", "Or Neither", ["gamma"])

    data = await mutable_mcp_env.json(
        "wiki_list", {"tags": ["alpha", "beta"], "tags_mode": "or", "format": "json"}
    )
    slugs = [p["slug"] for p in data["pages"]]
    assert "concepts/or-alpha" in slugs
    assert "concepts/or-beta" in slugs
    assert "concepts/or-neither" not in slugs


async def test_list_tags_empty_no_op(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    before = await mutable_mcp_env.json("wiki_list", {"format": "json"})
    total_before = before["total"]

    data = await mutable_mcp_env.json("wiki_list", {"tags": [], "format": "json"})
    assert data["total"] == total_before


async def test_list_min_confidence(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/high-conf", "High Conf", [], confidence=0.9)
    await _add_page(mutable_mcp_env, "concepts/low-conf", "Low Conf", [], confidence=0.2)

    data = await mutable_mcp_env.json(
        "wiki_list", {"min_confidence": 0.5, "format": "json"}
    )
    slugs = [p["slug"] for p in data["pages"]]
    assert "concepts/high-conf" in slugs
    assert "concepts/low-conf" not in slugs


async def test_list_confidence_absent_passes(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/no-conf-page", "No Conf Page", [], confidence=None)

    data = await mutable_mcp_env.json(
        "wiki_list", {"min_confidence": 0.9, "format": "json"}
    )
    slugs = [p["slug"] for p in data["pages"]]
    assert "concepts/no-conf-page" in slugs


async def test_list_sort_confidence_desc(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/conf-low", "Conf Low", [], confidence=0.2)
    await _add_page(mutable_mcp_env, "concepts/conf-high", "Conf High", [], confidence=0.95)
    await _add_page(mutable_mcp_env, "concepts/conf-mid", "Conf Mid", [], confidence=0.5)

    data = await mutable_mcp_env.json(
        "wiki_list", {"sort": "confidence", "order": "desc", "format": "json"}
    )
    pages = data["pages"]
    confs = [p["confidence"] for p in pages if p["slug"] in ("concepts/conf-low", "concepts/conf-high", "concepts/conf-mid")]
    assert confs == sorted(confs, reverse=True), f"expected desc confidence order, got: {confs}"


async def test_list_order_desc(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/zzz-last", "Zzz Last", [])
    await _add_page(mutable_mcp_env, "concepts/aaa-first", "Aaa First", [])

    asc = await mutable_mcp_env.json("wiki_list", {"sort": "slug", "order": "asc", "format": "json"})
    desc = await mutable_mcp_env.json("wiki_list", {"sort": "slug", "order": "desc", "format": "json"})

    asc_slugs = [p["slug"] for p in asc["pages"]]
    desc_slugs = [p["slug"] for p in desc["pages"]]
    assert asc_slugs == list(reversed(desc_slugs))


# ── wiki_search filters ───────────────────────────────────────────────────────


async def test_search_status_filter(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/active-page", "Active Filtertest Page", [], status="active")
    await _add_page(mutable_mcp_env, "concepts/draft-page", "Draft Filtertest Page", [], status="draft")

    data = await mutable_mcp_env.json(
        "wiki_search", {"query": "filtertest", "status": "active", "format": "json"}
    )
    slugs = [r["slug"] for r in data["results"]]
    assert "concepts/active-page" in slugs
    assert "concepts/draft-page" not in slugs


async def test_search_tags_filter_and(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/search-tag-ab", "Search Tag AB", ["search-alpha", "search-beta"])
    await _add_page(mutable_mcp_env, "concepts/search-tag-a", "Search Tag A Only", ["search-alpha"])

    data = await mutable_mcp_env.json(
        "wiki_search",
        {"query": "search tag", "tags": ["search-alpha", "search-beta"], "tags_mode": "and", "format": "json"},
    )
    slugs = [r["slug"] for r in data["results"]]
    assert "concepts/search-tag-ab" in slugs
    assert "concepts/search-tag-a" not in slugs


async def test_search_min_confidence(mutable_mcp_env):
    await mutable_mcp_env.rebuild()
    await _add_page(mutable_mcp_env, "concepts/search-high-conf", "Search High Conftest", [], confidence=0.9)
    await _add_page(mutable_mcp_env, "concepts/search-low-conf", "Search Low Conftest", [], confidence=0.1)

    data = await mutable_mcp_env.json(
        "wiki_search", {"query": "conftest", "min_confidence": 0.5, "format": "json"}
    )
    slugs = [r["slug"] for r in data["results"]]
    assert "concepts/search-high-conf" in slugs
    assert "concepts/search-low-conf" not in slugs
