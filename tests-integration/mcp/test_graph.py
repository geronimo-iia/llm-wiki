from conftest import SPACE_NAME, SPACE_NOTES, SLUG_MoE


async def test_graph_mermaid_output(mcp_env):
    await mcp_env.rebuild(SPACE_NAME)
    await mcp_env.rebuild(SPACE_NOTES)
    text = await mcp_env.call("wiki_graph")
    lower = text.lower()
    assert "graph lr" in lower or "graph td" in lower or "flowchart" in lower


async def test_graph_dot_output(mcp_env):
    await mcp_env.rebuild(SPACE_NAME)
    await mcp_env.rebuild(SPACE_NOTES)
    text = await mcp_env.call("wiki_graph", {"format": "dot"})
    assert "digraph" in text


async def test_graph_llms_output(mcp_env):
    await mcp_env.rebuild(SPACE_NAME)
    await mcp_env.rebuild(SPACE_NOTES)
    text = await mcp_env.call("wiki_graph", {"format": "llms"})
    lower = text.lower()
    assert "nodes" in lower or "edges" in lower or "type groups" in lower


async def test_graph_type_filter(mcp_env):
    await mcp_env.rebuild(SPACE_NAME)
    await mcp_env.rebuild(SPACE_NOTES)
    text = await mcp_env.call("wiki_graph", {"type": "concept"})
    # mermaid: node lines contain brackets e.g. N["label"]
    node_lines = [l for l in text.splitlines() if "[" in l and "]" in l]
    assert len(node_lines) > 0, "type-filtered graph must contain at least one concept node"


async def test_graph_root_depth(mcp_env):
    await mcp_env.rebuild(SPACE_NAME)
    await mcp_env.rebuild(SPACE_NOTES)
    text = await mcp_env.call("wiki_graph", {"root": SLUG_MoE, "depth": 2})
    node_lines = [l for l in text.splitlines() if "[" in l and "]" in l]
    assert len(node_lines) > 0, f"depth-2 subgraph rooted at {SLUG_MoE!r} must contain nodes"


async def test_graph_cross_wiki(mcp_env):
    await mcp_env.rebuild(SPACE_NAME)
    await mcp_env.rebuild(SPACE_NOTES)
    text = await mcp_env.call("wiki_graph", {"cross_wiki": True})
    node_lines = [l for l in text.splitlines() if "[" in l and "]" in l]
    assert len(node_lines) > 0, "cross-wiki graph must contain at least one node"
