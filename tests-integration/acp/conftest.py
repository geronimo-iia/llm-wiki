import asyncio
import json
from pathlib import Path


class AcpEnv:
    def __init__(self, binary: str, config: Path):
        self.binary = binary
        self.config = config
        self._req_id = 0

    def _next_id(self) -> int:
        self._req_id += 1
        return self._req_id

    def _request(self, method: str, params: dict) -> str:
        return json.dumps(
            {"jsonrpc": "2.0", "id": self._next_id(), "method": method, "params": params}
        )

    async def exchange(self, messages: list[str], timeout: float = 15.0) -> list[dict]:
        """Send NDJSON messages, return parsed response lines."""
        payload = "\n".join(messages) + "\n"
        proc = await asyncio.create_subprocess_exec(
            self.binary,
            "--config",
            str(self.config),
            "serve",
            "--acp",
            "--http",
            ":0",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL,
        )
        try:
            # Write payload and close stdin
            proc.stdin.write(payload.encode())
            await proc.stdin.drain()
            proc.stdin.close()

            # Read all available output up to timeout
            deadline = asyncio.get_event_loop().time() + timeout
            output_lines = []
            while asyncio.get_event_loop().time() < deadline:
                try:
                    # Read with shorter timeout and loop
                    remaining = deadline - asyncio.get_event_loop().time()
                    if remaining <= 0:
                        break
                    line = await asyncio.wait_for(
                        proc.stdout.readline(), timeout=min(0.5, remaining)
                    )
                    if not line:
                        # EOF reached
                        break
                    output_lines.append(line.decode())
                except TimeoutError:
                    # No more data coming soon, check if we have responses
                    if output_lines:
                        break
                    continue

            # Ensure process is terminated
            proc.kill()
            await proc.wait()

            stdout_text = "".join(output_lines)
        except TimeoutError:
            proc.kill()
            await proc.wait()
            raise
        result = []
        for i, line in enumerate(stdout_text.splitlines()):
            if not line.strip():
                continue
            try:
                result.append(json.loads(line))
            except json.JSONDecodeError as e:
                raise AssertionError(f"ACP response line {i} is not valid JSON: {e!r}") from e
        return result

    async def initialize(self) -> dict:
        """Send initialize, return its result dict."""
        self._req_id = 0
        msgs = [
            self._request(
                "initialize",
                {
                    "protocolVersion": 1,
                    "clientInfo": {"name": "acp-test", "version": "0.1.0"},
                },
            )
        ]
        responses = await self.exchange(msgs)
        return _find_result(responses, req_id=1)

    async def new_session(self, cwd: str, wiki: str | None = None) -> str:
        """init + session/new; returns sessionId."""
        self._req_id = 0
        meta = {"wiki": wiki} if wiki else None
        params: dict = {"cwd": cwd, "mcpServers": []}
        if meta:
            params["_meta"] = meta
        msgs = [
            self._request(
                "initialize",
                {
                    "protocolVersion": 1,
                    "clientInfo": {"name": "acp-test", "version": "0.1.0"},
                },
            ),
            self._request("session/new", params),
        ]
        responses = await self.exchange(msgs)
        result = _find_result(responses, req_id=2)
        sid = result.get("sessionId")
        if not sid:
            raise AssertionError(f"session/new returned no sessionId; result={result}")
        return sid

    async def prompt(self, cwd: str, text: str, wiki: str | None = None) -> tuple[list[dict], dict]:
        """Full round-trip: init → session/new → session/prompt.

        Returns (all_responses, prompt_result).
        prompt_result is the result of the session/prompt request.
        """
        self._req_id = 0
        meta = {"wiki": wiki} if wiki else None
        new_params: dict = {"cwd": cwd, "mcpServers": []}
        if meta:
            new_params["_meta"] = meta

        # Phase 1: init + new_session
        init_msgs = [
            self._request(
                "initialize",
                {
                    "protocolVersion": 1,
                    "clientInfo": {"name": "acp-test", "version": "0.1.0"},
                },
            ),
            self._request("session/new", new_params),
        ]
        init_responses = await self.exchange(init_msgs)
        new_result = _find_result(init_responses, req_id=2)
        sid = new_result.get("sessionId")
        if not sid:
            raise AssertionError(f"session/new returned no sessionId; result={new_result}")

        # Phase 2: prompt
        self._req_id = 0
        prompt_msgs = [
            self._request(
                "session/prompt",
                {
                    "sessionId": sid,
                    "prompt": [{"type": "text", "text": text}],
                },
            )
        ]
        prompt_responses = await self.exchange(prompt_msgs)
        prompt_result = _find_result(prompt_responses, req_id=1)
        return init_responses + prompt_responses, prompt_result

    async def session_list(self, cwd: str) -> list[dict]:
        """init + session/new + session/list; returns sessions array."""
        self._req_id = 0
        msgs = [
            self._request(
                "initialize",
                {
                    "protocolVersion": 1,
                    "clientInfo": {"name": "acp-test", "version": "0.1.0"},
                },
            ),
            self._request("session/new", {"cwd": cwd, "mcpServers": []}),
            self._request("session/list", {}),
        ]
        responses = await self.exchange(msgs)
        result = _find_result(responses, req_id=3)
        return result.get("sessions", [])

    async def session_load(self, cwd: str, session_id: str) -> dict:
        """init + session/load(session_id); returns the raw response dict (may contain 'result' or 'error')."""
        self._req_id = 0
        msgs = [
            self._request(
                "initialize",
                {
                    "protocolVersion": 1,
                    "clientInfo": {"name": "acp-test", "version": "0.1.0"},
                },
            ),
            self._request(
                "session/load",
                {
                    "sessionId": session_id,
                    "cwd": cwd,
                    "mcpServers": [],
                },
            ),
        ]
        responses = await self.exchange(msgs)
        for r in responses:
            if r.get("id") == 2:
                return r
        raise AssertionError(f"no response with id=2 in: {responses}")

    def collect_text(self, responses: list[dict]) -> str:
        """Collect streamed agent_message_chunk text from session/update notifications."""
        parts = []
        for r in responses:
            if r.get("method") != "session/update":
                continue
            update = r.get("params", {}).get("update", {})
            if update.get("sessionUpdate") == "agent_message_chunk":
                text = update.get("content", {}).get("text", "")
                if text:
                    parts.append(text)
        return "".join(parts)


def _find_result(responses: list[dict], req_id: int) -> dict:
    """Find the result for a given request id; raises AssertionError if not found or is error."""
    for r in responses:
        if r.get("id") == req_id:
            if "error" in r:
                raise AssertionError(f"request id={req_id} returned error: {r['error']}")
            return r.get("result", {})
    raise AssertionError(f"no response with id={req_id} in: {responses[:3]}")


def make_acp_env(wiki_env) -> AcpEnv:
    """Convenience factory for use in tests."""
    return AcpEnv(binary=wiki_env.binary, config=wiki_env.config)
