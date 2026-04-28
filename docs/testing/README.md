# Testing

End-to-end validation for the llm-wiki CLI, MCP server, and Claude plugin.

## Prerequisites

### CLI validation

| Tool | Install |
|------|---------|
| `jq` | `brew install jq` · `apt install jq` |
| `git` | system |

### MCP validation

The MCP validation scripts require **mcptools** — a Go CLI for calling MCP servers.
It handles the session protocol (initialize handshake, streamable HTTP) so the
scripts only need to call tools by name.

```bash
# macOS — Homebrew (recommended)
brew tap f/mcptools && brew install mcp

# Any platform — direct binary download
# https://github.com/f/mcptools/releases
# Download the archive for your platform, extract, place `mcp` on PATH

# Verify
mcp --version
```

> `mcptools` is a single Go binary. No Node.js, no Python required.

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/setup-test-env.sh` | Create the persistent test environment and export env vars |
| `scripts/clean-test-env.sh` | Remove the test environment and unset env vars |
| `scripts/validate-engine.sh` | CLI validation — all sections |
| `scripts/validate-mcp.sh` | MCP HTTP server validation — all sections |
| `scripts/sections/NN-*.sh` | CLI section scripts (sourced by validate-engine.sh) |
| `scripts/mcp/NN-*.sh` | MCP section scripts (sourced by validate-mcp.sh) |
| `scripts/lib/helpers.sh` | Shared `pass`/`fail`/`run`/`run_json` helpers |
| `scripts/lib/mcp-helpers.sh` | MCP-specific `run_mcp`/`run_mcp_json` helpers |

## Quick start — CLI

```bash
# 1. Build the binary
cargo build --release

# 2. Create the test environment and export env vars
source ./docs/testing/scripts/setup-test-env.sh

# 3. Run all CLI validations
LLM_WIKI_BIN=./target/release/llm-wiki \
./docs/testing/scripts/validate-engine.sh

# 4. Run a single section (e.g. ingest only)
LLM_WIKI_BIN=./target/release/llm-wiki \
./docs/testing/scripts/validate-engine.sh --section 05

# 5. Clean up when done
source ./docs/testing/scripts/clean-test-env.sh
```

## Quick start — MCP server

The MCP suite uses **stdio transport** via `mcptools` — no server process to
start or stop. Each `mcp call` launches the binary as a subprocess, performs
the MCP handshake, and exits.

```bash
# 1. Build the binary (if not already done)
cargo build --release

# 2. Create the test environment and export env vars (if not already done)
source ./docs/testing/scripts/setup-test-env.sh

# 3. Run all MCP validations
LLM_WIKI_BIN=./target/release/llm-wiki \
./docs/testing/scripts/validate-mcp.sh

# 4. Run a single section (e.g. search only)
LLM_WIKI_BIN=./target/release/llm-wiki \
./docs/testing/scripts/validate-mcp.sh --section 03
```

`source` is required for `setup-test-env.sh` and `clean-test-env.sh` so
that `LLM_WIKI_TEST_DIR` and `LLM_WIKI_CONFIG` are exported/unset in
your current shell. Running them directly also works but won't affect the
parent shell's environment.

## GitHub Actions

A `workflow_dispatch` workflow is available at
`.github/workflows/integration.yml`. Trigger it manually from the Actions
tab with a choice of suite (`both` / `engine` / `mcp`). It builds the binary,
installs mcptools, sets up the test environment, and runs the selected suite.

## Skills validation

Open `docs/testing/validate-skills.md` and run each scenario in Claude
with the plugin active. Both wikis must be registered before starting
(run `setup-test-env.sh` first or register them manually).

---

## Deliberate fixtures for lint rules

| Page | Rule triggered | Why |
|---|---|---|
| `concepts/orphan-concept.md` | `orphan` | No inbound or outbound links |
| `concepts/broken-link-concept.md` | `broken-link` | `concepts` field references `concepts/does-not-exist` |
| `concepts/compute-efficiency.md` | `stale` (over time) | Low confidence draft |

## Deliberate contradictions

- `concepts/sparse-routing` claims compute cost is O(k/n)
- `concepts/compute-efficiency` draft claims compute cost is O(n)
- `02-article-moe-efficiency.md` also argues the O(k/n) claim is misleading

These are intentional for testing the ingest analysis step (imp-11) and
the review skill (imp-12).

---

## Source layout

Static test data committed to the repo:

```
tests/fixtures/
  wikis/
    research/                ← primary test wiki (MoE / transformer domain)
      wiki.toml              ← sets min_nodes_for_communities=5 for community tests
      wiki/
        concepts/            ← 6 concept pages (includes orphan + broken-link fixtures)
        sources/             ← 1 source page
        inbox/               ← empty placeholder; setup copies inbox/ files here
    notes/                   ← second wiki for cross-wiki tests
      wiki.toml
      wiki/
        concepts/            ← 1 concept page (attention-mechanism, cross-wiki target)
  inbox/
    01-paper-switch-transformer.md  ← rich paper; tests ingest + contradiction detection
    02-article-moe-efficiency.md    ← article; tests claim contradiction with sparse-routing
    03-note-with-secrets.md         ← contains fake API keys; tests redaction
    04-note-cross-wiki.md           ← contains wiki:// links; tests cross-wiki
    05-data-benchmark-scores.csv    ← CSV; tests data source type classification
```

## Testing layout

What `setup-test-env.sh` creates at `~/llm-wiki-testing/` (or `--dir` path):

```
~/llm-wiki-testing/          ← LLM_WIKI_TEST_DIR
  config.toml                ← LLM_WIKI_CONFIG — isolated engine config (space registry)
  wikis/
    research/                ← copy of tests/fixtures/wikis/research
      wiki.toml
      wiki/
        concepts/
        sources/
        inbox/               ← inbox fixtures copied here before each validation run
    notes/                   ← copy of tests/fixtures/wikis/notes
      wiki.toml
      wiki/
        concepts/
```

Export files (`wiki_export`) and other validation artefacts are written
directly to `~/llm-wiki-testing/` so you can inspect them after a run.
