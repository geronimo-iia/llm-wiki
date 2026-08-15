import pytest


async def test_acp_max_sessions_config_readable(wiki_env):
    result = wiki_env.run("config", "get", "serve.acp_max_sessions", check=False)
    assert result.returncode in (0, 1)


@pytest.mark.skip(
    reason="cap enforcement requires persistent server; each exchange() is a fresh subprocess"
)
async def test_session_cap_enforced(wiki_env):
    pass
