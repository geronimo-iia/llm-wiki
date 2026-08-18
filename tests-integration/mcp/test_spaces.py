import tempfile

from conftest import SPACE_NAME


async def test_spaces_list_returns_research(mcp_env):
    text = await mcp_env.call("wiki_spaces_list")
    assert SPACE_NAME in text


async def test_spaces_list_json_contains_research(mcp_env):
    data = await mcp_env.json("wiki_spaces_list")
    assert any(w["name"] == SPACE_NAME for w in data)


async def test_spaces_set_default_research(mcp_env):
    text = await mcp_env.call("wiki_spaces_set_default", {"name": SPACE_NAME})
    assert SPACE_NAME in text


async def test_spaces_set_default_updates_in_memory_engine(mutable_mcp_env):
    # Switch default to notes, then wiki_list without wiki: param must return notes pages, not research pages.
    # This exercises the in-memory engine default, not just the success text.
    from conftest import SPACE_NOTES
    await mutable_mcp_env.rebuild(SPACE_NOTES)
    await mutable_mcp_env.call("wiki_spaces_set_default", {"name": SPACE_NOTES})
    data = await mutable_mcp_env.json("wiki_list", {"format": "json"})
    slugs = [p["slug"] for p in data.get("pages", [])]
    # concepts/attention-mechanism exists only in notes, not in research
    assert any("attention-mechanism" in s for s in slugs), (
        f"wiki_list without wiki: param should return notes pages after set_default, got: {slugs}"
    )


async def test_spaces_register_rollback_on_mount_failure(mutable_mcp_env):
    # A real directory that is not a git repo — mount_wiki fails (no HEAD), config entry must be rolled back
    with tempfile.TemporaryDirectory() as not_a_git_repo:
        is_error, text = await mutable_mcp_env.call_raw(
            "wiki_spaces_register",
            {"path": not_a_git_repo, "name": "rollback-test"},
        )
    assert is_error, f"expected error from register on non-git path, got: {text!r}"
    data = await mutable_mcp_env.json("wiki_spaces_list")
    names = [w["name"] for w in data]
    assert "rollback-test" not in names, "rollback failed: name still in config after mount failure"
