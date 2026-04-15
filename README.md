# llm-wiki

Git-backed wiki engine with MCP server — bring your own LLM.

Implements Karpathy's LLM Wiki concept: process sources at **ingest time** rather than
query time, building a persistent, cross-referenced knowledge base where contradictions
are first-class knowledge nodes.

`llm-wiki` is the **engine**, not the LLM. It manages structured Markdown, git history,
full-text search, and MCP exposure. Your external LLM reads sources, produces
`analysis.json`, and feeds it to the wiki.

---

## Install

```bash
cargo install llm-wiki
```

Minimum Rust version: **1.75** (edition 2021, async traits via `async-trait`).

---

## Quick start

```bash
# 1. Create a directory and let wiki ingest initialise git automatically
mkdir my-wiki && cd my-wiki

# 2. Have your external LLM analyse a source and produce analysis.json
my-llm analyze paper.pdf > analysis.json

# 3. Ingest — writes pages, commits to git
wiki ingest analysis.json

# Or pipe directly from the LLM
my-llm analyze paper.pdf | wiki ingest -
```

---

## CLI reference

Global flags (work with every subcommand):

| Flag | Description |
|---|---|
| `--wiki <name>` | Target a specific registered wiki by name (default: wiki marked `default = true` in `~/.wiki/config.toml`) |

Subcommands:

| Command | Description |
|---|---|
| `wiki ingest <file>` | Integrate an `analysis.json` file into the wiki |
| `wiki ingest -` | Read `analysis.json` from stdin |
| `wiki search "<term>"` | Full-text BM25 search — prints ranked results table |
| `wiki search "<term>" --top <n>` | Limit results (default 20) |
| `wiki search "<term>" --all` | Fan out to all registered wikis; adds WIKI column |
| `wiki search --rebuild-index` | Rebuild the tantivy index and exit |
| `wiki context "<question>"` | Return top-K relevant pages as Markdown for an LLM |
| `wiki context "<question>" --top-k <n>` | Control page count (default 5) |
| `wiki lint` | Structural audit — writes `LINT.md`, commits |
| `wiki list` | List all pages as a table (slug, title, type) |
| `wiki list --type concept\|source\|contradiction\|query` | Filter by page type |
| `wiki contradict` | List all contradiction pages (slug, title, status, dimension) |
| `wiki contradict --status active\|resolved\|under-analysis` | Filter by status |
| `wiki graph` | Print the concept graph as GraphViz DOT to stdout |
| `wiki graph --format mermaid` | Print as Mermaid instead |
| `wiki diff` | Print the diff of the last commit (git diff HEAD~1) |
| `wiki init [<path>]` | Initialise a new wiki repo (git init + directory scaffold) |
| `wiki init [<path>] --register` | Also register the new wiki in `~/.wiki/config.toml` |
| `wiki read <slug>` | Print full content of a page (frontmatter + body) |
| `wiki read <slug> --body-only` | Print body only, no frontmatter |
| `wiki serve` | Start the MCP server (stdio transport) |
| `wiki serve --sse :<port>` | Start the MCP server on HTTP SSE transport |
| `wiki instruct [<workflow>]` | Print LLM usage instructions (full or named section) |

Exit code is **0** on success, **1** on any validation or write error.

### End-to-end example

```bash
# 1. External LLM analyses a source and produces analysis.json
my-llm analyze paper.pdf > analysis.json

# 2. Ingest — writes pages, commits to git
wiki ingest analysis.json

# 3. Retrieve context for a follow-up question
wiki context "how does MoE scaling work?" | my-llm answer

# 4. Search for specific concepts
wiki search "mixture of experts" --top 5

# 5. Run a structural lint pass
wiki lint

# 6. See what the last ingest changed
wiki diff

# 7. List all concept pages
wiki list --type concept

# 8. List unresolved contradictions
wiki contradict --status active

# 9. Visualise the concept graph
wiki graph | dot -Tsvg -o graph.svg
```

On a fresh clone, rebuild the search index first:

```bash
git clone <wiki-repo> my-wiki && cd my-wiki
wiki search --rebuild-index
wiki search "scaling laws"
```

---

## Multi-wiki

Manage multiple independent wiki repositories from a single `wiki` install.

### Setup

Create `~/.wiki/config.toml`:

```toml
[[wikis]]
name    = "work"
path    = "/Users/me/work-wiki"
default = true

[[wikis]]
name   = "research"
path   = "/Users/me/research-wiki"
remote = "git@github.com:me/research-wiki.git"   # optional
```

Or let `wiki init --register` build it incrementally:

```bash
wiki init ~/work-wiki --register     # creates work-wiki and adds it as default
wiki init ~/research-wiki --register # adds research-wiki
```

### Targeting a specific wiki

```bash
wiki --wiki research ingest analysis.json
wiki --wiki work search "transformer"
wiki --wiki research lint
```

Omitting `--wiki` uses the wiki marked `default = true`.  If no config file
exists the current directory is used as a single-wiki fallback.

### Cross-wiki search

```bash
wiki search --all "attention mechanism"
```

Fans out to every registered wiki, merges results by BM25 score, and displays
a `WIKI` column so you can see which repository each result came from.

---

## SSE transport

The MCP server supports HTTP Server-Sent Events in addition to stdio, letting
multiple clients (or remote agents) connect simultaneously.

```bash
wiki serve --sse :8080
```

The server listens on `0.0.0.0:8080`.  Each client that connects to
`GET /sse` receives an independent `WikiServer` session with no shared mutable
state.  Messages are sent via `POST /message?sessionId=<id>`.

**With a registry:**

```bash
wiki --wiki research serve --sse :8080
```

Every session can target named wikis via the `wiki` tool parameter.

**When to prefer SSE over stdio:**

| | stdio | SSE |
|---|---|---|
| Single Claude Code client | ✅ simpler | works |
| Multiple simultaneous clients | ✗ | ✅ |
| Remote / containerised agents | awkward | ✅ |

Graceful shutdown: send `Ctrl-C` to the server process.

---

## Claude Code plugin

`llm-wiki` ships as a Claude Code plugin with six slash commands.

### Install

**Local install** (development / private wiki):

```bash
claude plugin add /path/to/llm-wiki
```

**Marketplace install**:

```bash
claude plugin marketplace add geronimo-iia/llm-wiki
```

### Post-install

Run the setup command in a new Claude Code session:

```
/llm-wiki:init
```

This verifies `wiki` is installed, runs `wiki init`, and writes the MCP server
config to `~/.claude/settings.json`.

### Slash commands

| Command | Description |
|---|---|
| `/llm-wiki:help` | List all commands, MCP tools, and workflow links |
| `/llm-wiki:init` | Verify install, initialise wiki repo, configure MCP |
| `/llm-wiki:ingest` | Ingest a source — two-step: wiki_context then wiki_ingest |
| `/llm-wiki:research` | Answer a question from wiki knowledge via wiki_context |
| `/llm-wiki:lint` | Run wiki_lint, fix orphans, enrich contradictions |
| `/llm-wiki:contradiction` | Analyse and enrich contradiction pages |

Each slash command fetches live workflow instructions from the `wiki` binary
(`wiki instruct <name>`) so the instructions stay in sync with the installed
version — no plugin update required when the instructions change.

---

## MCP server

`wiki` ships a built-in [Model Context Protocol](https://modelcontextprotocol.io)
server. Add it to your Claude Code `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "wiki": {
      "command": "wiki",
      "args": ["serve"],
      "cwd": "/path/to/your/wiki"
    }
  }
}
```

Then `cd` into your wiki directory and start the server:

```bash
cd my-wiki
wiki serve
```

### MCP tools

| Tool | Description |
|---|---|
| `wiki_ingest` | Ingest an `analysis.json` object — returns a page-count summary |
| `wiki_context` | Return top-K relevant pages as Markdown for a question |
| `wiki_search` | BM25 full-text search — returns JSON array of `{slug, title, snippet, score}` |
| `wiki_lint` | Structural audit — returns JSON summary of orphans, stubs, active contradictions |
| `wiki_list` | List pages, optionally filtered by type |

### MCP resources

Pages are exposed as `wiki://default/{type}/{slug}` URIs:

```
wiki://default/concepts/mixture-of-experts
wiki://default/sources/switch-transformer-2021
wiki://default/contradictions/moe-scaling-efficiency
wiki://default/queries/how-does-moe-work
```

### MCP prompts

| Prompt | Arguments | Description |
|---|---|---|
| `ingest_source` | `source` | Step-by-step workflow to ingest a new source |
| `research_question` | `question`, `save?` | Retrieve context and answer a question |
| `lint_and_enrich` | — | Run lint and address findings |
| `analyse_contradiction` | `slug` | Deep-dive into a contradiction page |

### LLM instructions

```bash
wiki instruct            # full instructions
wiki instruct ingest     # ingest-workflow section only
wiki instruct research   # research-workflow section only
```

---

### Coming in later phases

| Command | Phase | Description |
|---|---|---|
| `wiki serve --sse :<port>` | 6 | HTTP SSE transport for multi-client scenarios |

---

## Contradictions

Contradictions are **first-class knowledge nodes** — never deleted, only enriched.

When the external LLM detects a contradiction between a new source and an existing
wiki page, it records it in `analysis.json`. `wiki ingest` writes a
`contradictions/<slug>.md` page with the full schema:

```yaml
title: "MoE scaling efficiency: contradictory views"
type: contradiction
claim_a: "sparse MoE reduces effective compute 8x at same quality"
source_a: "sources/switch-transformer-2021"
claim_b: "MoE gains diminish sharply beyond 100B parameters"
source_b: "sources/moe-survey-2023"
dimension: context     # context | time | scale | methodology | open-dispute
status: active         # active | resolved | under-analysis
epistemic_value: >
  Compute/quality tradeoff in MoE is phase-dependent.
```

`wiki lint` surfaces unresolved contradictions in `LINT.md`. The external LLM
reads `LINT.md`, enriches each active contradiction (adds `resolution:`, sets
`status: resolved`), and re-ingests. The resolved contradiction stays in the wiki
forever — the resolution *is* the knowledge.

`git log contradictions/<slug>.md` traces how understanding of that tension evolved.

See [`docs/dev/contradictions.md`](docs/dev/contradictions.md) for the full
lifecycle and enrichment workflow.

---

## The `analysis.json` contract

The external LLM produces this JSON after reading a source document.
`wiki ingest` validates and integrates it — no LLM calls inside the binary.

```json
{
  "source": "path/or/url/to/original",
  "doc_type": "research-paper",
  "title": "Switch Transformer",
  "language": "en",
  "claims": [
    { "text": "sparse MoE reduces compute 8x", "confidence": "high", "section": "Results" }
  ],
  "concepts": ["mixture-of-experts", "scaling-laws"],
  "key_quotes": ["...verbatim quote..."],
  "data_gaps": ["no fine-tuning evaluation"],
  "suggested_pages": [
    {
      "slug": "concepts/mixture-of-experts",
      "title": "Mixture of Experts",
      "type": "concept",
      "action": "create",
      "tldr": "Sparse routing of tokens to expert subnetworks.",
      "body": "## Overview\n\nMoE routes tokens to sparse expert subnetworks…",
      "tags": ["transformers", "scaling"],
      "read_when": ["Reasoning about MoE architecture"]
    }
  ],
  "contradictions": []
}
```

### `doc_type` values

`research-paper` | `blog-post` | `transcript` | `thread` | `note` | `book-chapter`

### `action` semantics

| `action` | Precondition | Effect |
|---|---|---|
| `create` | slug must **not** exist | Writes `{slug}.md` with generated frontmatter and body |
| `update` | slug **must** exist | Replaces body; merges frontmatter (union tags, overwrite title/tldr) |
| `append` | slug **must** exist | Appends new Markdown section after existing body, separated by `---` |

### Slug format

Slugs are paths relative to the wiki root without the `.md` extension.
Only four prefixes are accepted: `concepts/`, `sources/`, `queries/`, `contradictions/`.
Path traversal (`../`) is rejected.

Full contract: [`docs/design/design.md`](docs/design/design.md)

---

## Repository layout

Pages exist in two forms. A slug like `concepts/foo` resolves to whichever
form is on disk — all commands handle both transparently.

**Flat** — page with no assets:
```
concepts/foo.md
```

**Bundle** — page with co-located assets:
```
concepts/foo/
├── index.md          ← the page
├── diagram.png       ← asset co-located beside the page
└── config.yaml
```

Full wiki tree:

```
my-wiki/                    ← git root (auto-initialised by wiki ingest)
├── concepts/               ← concept pages (flat or bundle)
├── sources/                ← per-source summary pages
├── contradictions/         ← contradiction nodes (always flat)
├── queries/                ← saved Q&A results (always flat)
├── assets/                 ← shared assets only (referenced by 2+ pages)
│   ├── index.md            ← auto-generated table of shared assets
│   ├── diagrams/
│   ├── configs/
│   └── scripts/
└── .wiki/
    └── config.toml
```

Every page file: `---\n<yaml frontmatter>\n---\n\n<Markdown body>`

**When to use each form:**
- Flat — page has no assets, or assets are short enough to inline as fenced blocks.
- Bundle — page has one or more files worth preserving (diagrams, configs, scripts).
  Assets live beside `index.md` and are referenced with short relative paths
  (`./diagram.png`). The wiki promotes a flat page to a bundle automatically
  when the first asset is written.
- Central `assets/` — only for assets explicitly shared across multiple pages.
  Prefer co-location; use `assets/` as the exception.

See [`docs/design/repository-layout.md`](docs/design/repository-layout.md) for
the full layout rationale and [`docs/dev/layout.md`](docs/dev/layout.md) for
the implementation details (slug resolution, bundle promotion, walker rules).

---

## Design

See [docs/design/](docs/design/) for the full architecture and `analysis.json` contract.
See [docs/dev/](docs/dev/) for module-level implementation notes.

## License

MIT OR Apache-2.0
