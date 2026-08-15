import pytest

from acp.conftest import make_acp_env


async def test_initialize_returns_agent_name(wiki_env):
    acp = make_acp_env(wiki_env)
    result = await acp.initialize()
    assert result.get("agentInfo", {}).get("name") == "llm-wiki"


async def test_session_new_returns_session_id(wiki_env):
    acp = make_acp_env(wiki_env)
    sid = await acp.new_session(cwd=str(wiki_env.tmp))
    assert isinstance(sid, str) and len(sid) > 0


async def test_session_new_with_wiki_meta_returns_session_id(wiki_env):
    acp = make_acp_env(wiki_env)
    sid = await acp.new_session(cwd=str(wiki_env.tmp), wiki="research")
    assert isinstance(sid, str) and len(sid) > 0


@pytest.mark.skip(
    reason="session/load requires persistent server; each exchange() is a fresh subprocess"
)
async def test_session_load_existing_succeeds(wiki_env):
    acp = make_acp_env(wiki_env)
    sid = await acp.new_session(cwd=str(wiki_env.tmp), wiki="research")
    resp = await acp.session_load(cwd=str(wiki_env.tmp), session_id=sid)
    assert "result" in resp


async def test_session_load_unknown_returns_error(wiki_env):
    acp = make_acp_env(wiki_env)
    resp = await acp.session_load(cwd=str(wiki_env.tmp), session_id="session-does-not-exist")
    assert "error" in resp
    assert "not found" in resp["error"].get("message", "").lower()


async def test_session_list_returns_at_least_one(wiki_env):
    acp = make_acp_env(wiki_env)
    sessions = await acp.session_list(cwd=str(wiki_env.tmp))
    assert len(sessions) >= 1
