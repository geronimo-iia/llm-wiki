async def test_ingest_dry_run_pages_validated(mcp_env):
    data = await mcp_env.json(
        "wiki_ingest",
        {"path": "inbox/01-paper-switch-transformer.md", "dry_run": True},
    )
    assert data["pages_validated"] >= 0


async def test_ingest_dry_run_has_warnings_array(mcp_env):
    data = await mcp_env.json(
        "wiki_ingest",
        {"path": "inbox/01-paper-switch-transformer.md", "dry_run": True},
    )
    assert isinstance(data["warnings"], list)


async def test_ingest_dry_run_unchanged_count(mcp_env):
    data = await mcp_env.json(
        "wiki_ingest",
        {"path": "inbox/01-paper-switch-transformer.md", "dry_run": True},
    )
    assert data["unchanged_count"] >= 0


async def test_ingest_dry_run_does_not_commit(mcp_env):
    # dry_run=True must validate pages but produce no git commit.
    data = await mcp_env.json(
        "wiki_ingest",
        {"path": "inbox/01-paper-switch-transformer.md", "dry_run": True},
    )
    assert data["pages_validated"] >= 1
    assert data["commit"] == "", (
        "dry_run ingest must not produce a commit; got commit=" + repr(data["commit"])
    )


async def test_ingest_redact_dry_run(mcp_env):
    data = await mcp_env.json(
        "wiki_ingest",
        {"path": "inbox/03-note-with-secrets.md", "dry_run": True, "redact": True},
    )
    assert data["pages_validated"] >= 0
