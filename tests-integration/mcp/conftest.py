import asyncio
import json

import pytest
import pytest_asyncio
from mcp.client.stdio import stdio_client

from mcp import ClientSession, StdioServerParameters

# Stable slugs/names from the research wiki fixture — update here if fixture changes
SLUG_MoE = "concepts/mixture-of-experts"
SLUG_SCALING_LAWS = "concepts/scaling-laws"
SLUG_ORPHAN = "concepts/orphan-concept"
SLUG_MISSING = "concepts/does-not-exist-xyz"
SPACE_NAME = "research"
SPACE_NOTES = "notes"


class McpEnv:
    def __init__(self, session: ClientSession):
        self._session = session

    async def call(self, tool: str, args: dict | None = None) -> str:
        result = await self._session.call_tool(tool, args or {})
        if not result.content:
            raise AssertionError(f"call_tool({tool!r}) returned empty content")
        if getattr(result, "is_error", False):
            raise AssertionError(f"call_tool({tool!r}) returned error: {result.content[0].text}")
        return result.content[0].text

    async def json(self, tool: str, args: dict | None = None) -> dict | list:
        text = await self.call(tool, args)
        return json.loads(text)

    async def rebuild(self, wiki: str = "research") -> None:
        await self.call("wiki_index_rebuild", {"wiki": wiki})

    async def call_raw(self, tool: str, args: dict | None = None) -> tuple[bool, str]:
        """Returns (is_error, text) without raising on tool errors."""
        result = await self._session.call_tool(tool, args or {})
        is_error = getattr(result, "is_error", False)
        text = result.content[0].text if result.content else ""
        return is_error, text


@pytest_asyncio.fixture()
async def mcp_env(wiki_env):
    """Standard read-only MCP session. Use mutable_mcp_env for tests that write wiki state."""
    server = StdioServerParameters(
        command=wiki_env.binary,
        args=["--config", str(wiki_env.config), "serve"],
    )

    ready: asyncio.Future = asyncio.get_event_loop().create_future()
    stop: asyncio.Event = asyncio.Event()
    env_holder: list[McpEnv] = []
    exc_holder: list[BaseException] = []

    async def _run():
        try:
            async with stdio_client(server) as (read, write), ClientSession(read, write) as session:
                await session.initialize()
                env_holder.append(McpEnv(session))
                ready.set_result(None)
                await stop.wait()
        except Exception as e:
            if not ready.done():
                ready.set_exception(e)
            else:
                exc_holder.append(e)

    task = asyncio.ensure_future(_run())
    await ready

    yield env_holder[0]

    stop.set()
    await task

    if exc_holder:
        raise exc_holder[0]


@pytest_asyncio.fixture()
async def mutable_mcp_env(wiki_env):
    """Same as mcp_env but signals that the test will mutate wiki state."""
    server = StdioServerParameters(
        command=wiki_env.binary,
        args=["--config", str(wiki_env.config), "serve"],
    )

    ready: asyncio.Future = asyncio.get_event_loop().create_future()
    stop: asyncio.Event = asyncio.Event()
    env_holder: list[McpEnv] = []
    exc_holder: list[BaseException] = []

    async def _run():
        try:
            async with stdio_client(server) as (read, write), ClientSession(read, write) as session:
                await session.initialize()
                env_holder.append(McpEnv(session))
                ready.set_result(None)
                await stop.wait()
        except Exception as e:
            if not ready.done():
                ready.set_exception(e)
            else:
                exc_holder.append(e)

    task = asyncio.ensure_future(_run())
    await ready

    yield env_holder[0]

    stop.set()
    await task

    if exc_holder:
        raise exc_holder[0]
