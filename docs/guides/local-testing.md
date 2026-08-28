# Local Testing Guide

How to test `llm-wiki-engine` and `llm-wiki-skills` on a laptop before publishing to GitHub or crates.io.

## Engine (binary)

Build the release binary:

```bash
cargo build --release
```

Output: `target/release/llm-wiki`

Claude Code resolves `llm-wiki` via `PATH`. The plugin's `.mcp.json` calls `llm-wiki serve`.
To shadow the homebrew-installed binary with the local build, symlink it earlier in PATH:

```bash
ln -sf "$(pwd)/target/release/llm-wiki" ~/.local/bin/llm-wiki
```

Verify the right binary is picked up:

```bash
which llm-wiki          # should show ~/.local/bin/llm-wiki
llm-wiki --version      # should match Cargo.toml version
```

Start a new Claude Code session — `wiki_info` will report the version from the local binary.

To restore the homebrew version:

```bash
rm ~/.local/bin/llm-wiki
```

## Skills (Claude plugin)

The `llm-wiki` plugin is installed from GitHub (`geronimo-iia/llm-wiki-skills`).
The cached copy lives at:

```
~/.claude/plugins/cache/geronimo-iia/llm-wiki/<version>/
```

To test local skill changes, point the plugin registry at a local path instead.
Edit `~/.claude/plugins/installed_plugins.json` — change the `llm-wiki@geronimo-iia` entry:

```json
"llm-wiki@geronimo-iia": [
  {
    "scope": "user",
    "installPath": "/absolute/path/to/llm-wiki-skills",
    "version": "local",
    "installedAt": "2026-01-01T00:00:00.000Z",
    "lastUpdated": "2026-01-01T00:00:00.000Z"
  }
]
```

Claude Code reads skills from `installPath` directly — no cache copy, no publish step.
Skill file edits are live in the next Claude Code session (no restart needed for new sessions).

To restore the published version, run:

```bash
claude plugin install geronimo-iia/llm-wiki-skills
```

That re-downloads the latest published version and updates the registry entry.

## Verifying the MCP server

After starting a Claude Code session with the local binary:

```
wiki_info
```

Check `version` matches `Cargo.toml`. Check `spaces` lists your registered wikis.

For a quick smoke test without Claude Code, run the MCP server manually and send a raw request:

```bash
target/release/llm-wiki serve &
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | target/release/llm-wiki serve
```

## Rust integration tests

The full test suite does not require the MCP server or any installed binary:

```bash
cargo test                            # all unit + integration tests
cargo test --test cli                 # CLI binary tests (uses CARGO_BIN_EXE_llm-wiki)
cargo test --test mcp                 # MCP tool smoke tests
```

Python end-to-end tests (requires `uv`):

```bash
make -C tests-integration test-engine
```
