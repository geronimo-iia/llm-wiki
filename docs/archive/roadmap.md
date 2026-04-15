# Implementation Roadmap

Each phase is independently shippable. A phase is "done" when its deliverables
compile, pass CI, and work end-to-end with real inputs.

Detailed task lists live in [docs/tasks/](tasks/).

---

## Phase 0 — Skeleton (compile-green baseline)

**Goal:** Everything compiles. CI is green. Schema structs are locked.

**Deliverables:**
- All `src/*.rs` modules compile with typed signatures (stubs return `todo!()`)
- `Analysis`, `SuggestedPage`, `Contradiction`, `PageFrontmatter` serde structs defined
  in `analysis.rs` and `markdown.rs` — the contracts everything else builds on
- `config.rs`: per-wiki `.wiki/config.toml` loads
- `Cargo.toml` dependency set finalized (no `rig-core`)
- CI: `cargo check`, `cargo clippy`, `cargo test` (empty integration tests pass)

**Why first:** Locking the data model early prevents rework. Every later phase
depends on `Analysis` and `PageFrontmatter` being stable.

Tasks: [tasks/phase-0.md](tasks/phase-0.md)

---

## Phase 1 — Core Write Loop

**Goal:** `wiki ingest analysis.json` works end-to-end. Pages appear on disk, committed.

**Deliverables:**
- `markdown.rs`: frontmatter parse + write (`serde_yaml` + `comrak`)
  - Round-trip fidelity: parse → write → parse produces identical struct
- `integrate.rs`:
  - `suggested_pages` → write `concepts/`, `sources/`, `queries/` Markdown files
  - `contradictions[]` → write `contradictions/` Markdown files if present
    (silently stored; no surfacing tooling yet — that is Phase 3)
  - `action: create|update|append` semantics for existing pages
- `git.rs`: `git2` commit with message `"ingest: <title> — +N pages"`
- `ingest.rs`: deserialize `analysis.json` → validate → `integrate` → `git commit`
- `cli.rs` + `main.rs`: `wiki ingest <file|->` wired up

**What is NOT in Phase 1:**
- No search (Phase 2)
- No contradiction surfacing commands (Phase 3)
- No `wiki contradict`, `wiki lint`, `wiki graph` (Phase 3)
- No MCP server (Phase 4)

**Acceptance test:**
```bash
echo '{"source":"test","doc_type":"note","title":"Test","claims":[],"concepts":[],"suggested_pages":[{"slug":"concepts/test","title":"Test","type":"concept","action":"create","tldr":"...","body":"...","tags":[]}],"contradictions":[]}' \
  | wiki ingest -
# → concepts/test.md exists with correct frontmatter
# → git log shows commit "ingest: Test — +1 pages"
```

Tasks: [tasks/phase-1.md](tasks/phase-1.md)

---

## Phase 2 — Search + Context

**Goal:** `wiki search` and `wiki context` work. External LLM can retrieve pages.

**Deliverables:**
- `search.rs`:
  - `tantivy` schema: fields for `slug`, `title`, `tags`, `body`, `type`
  - Index built from all `.md` files under the wiki root
  - `search(query) → Vec<SearchResult>` (BM25 ranked)
  - `--rebuild-index` flag to regenerate from scratch
  - Index stored in `.wiki/search-index/` (gitignored — rebuilt on demand)
- `context.rs`:
  - `context(question, top_k) → String` — runs search, fetches page bodies,
    formats as Markdown block ready for an LLM context window
- `cli.rs`: `wiki search "<term>"` + `wiki context "<question>"`

**Acceptance test:**
```bash
wiki ingest paper.json
wiki search "mixture of experts"       # → list of matching page slugs
wiki context "how does MoE work?"      # → Markdown with top-K page bodies
```

Tasks: [tasks/phase-2.md](tasks/phase-2.md)

---

## Phase 3 — Graph + Lint + Contradiction Surfacing

**Goal:** Structural quality signals. `wiki lint` produces an actionable report.
Contradiction pages written since Phase 1 are now queryable and clustered.

**Deliverables:**
- `graph.rs`:
  - Build `petgraph::DiGraph` from `[[wikilinks]]` and `related_concepts` frontmatter
  - Orphan detection: nodes with in-degree = 0 (excluding `raw/`)
  - `wiki graph` → DOT output (pipe to `dot -Tsvg`)
- `contradiction.rs`:
  - `list(status_filter) → Vec<ContradictionSummary>`
  - `cluster()` — petgraph subgraph of pages connected to active contradictions
- `lint.rs`:
  - Orphans, missing stubs (referenced but no file), active contradictions
  - Write `LINT.md`
  - `git commit -m "lint: <date>"`
- `cli.rs`: `wiki lint`, `wiki contradict [--status active|resolved]`,
  `wiki graph`, `wiki list [--type ...]`, `wiki diff`

**Note:** contradiction pages may already exist from Phase 1 ingests. Phase 3 adds
the commands to surface, query, and cluster them — not the write path.

**Acceptance test:**
```bash
wiki lint
# → LINT.md committed, reports orphans + active contradictions
wiki contradict --status active
# → table of unresolved contradictions
```

Tasks: [tasks/phase-3.md](tasks/phase-3.md)

---

## Phase 4 — MCP Server

**Goal:** `wiki serve` works inside Claude Code. All tools + resources + prompts live.

**Deliverables:**
- `server.rs` with `rmcp`:
  - Tools: `wiki_ingest`, `wiki_context`, `wiki_search`, `wiki_lint`, `wiki_list`
  - Resources: `wiki://{wiki}/{type}/{slug}` URIs
  - Resource notifications: `notify_resource_updated` on every ingest
  - Prompts: `ingest_source`, `research_question`, `lint_and_enrich`,
    `analyse_contradiction`
  - `src/instructions.md` embedded via `include_str!` and injected at connection time
- `cli.rs`: `wiki serve [--sse :<port>]`, `wiki instruct [<workflow>]`
- MCP config snippet in `.claude-plugin/.mcp.json` already present

**Acceptance test:**
```
# In Claude Code with wiki MCP server configured:
wiki_context(question: "how does MoE scaling work?")  → returns page bodies
wiki_ingest(analysis: {...})                          → pages committed, notification fired
```

Tasks: [tasks/phase-4.md](tasks/phase-4.md)  |  [tasks/schema-resource.md](tasks/schema-resource.md)

---

## Phase 5 — Claude Plugin

**Goal:** The `.claude-plugin/` directory is complete and installable.

**Deliverables:**
- `.claude-plugin/plugin.json`, `marketplace.json`, `.mcp.json` finalized
- Commands wired to `wiki instruct <command>` via `SKILL.md`
- `wiki instruct` returns correct step-by-step workflow for each command:
  `help`, `init`, `ingest`, `research`, `lint`, `contradiction`
- `src/instructions.md` covers all six workflows
- Install path verified: `claude plugin add /path/to/llm-wiki` → `/llm-wiki:ingest` works

**Acceptance test:**
```bash
claude plugin add /path/to/llm-wiki
# In Claude Code:
/llm-wiki:ingest
# → LLM fetches wiki instruct ingest, follows workflow, calls wiki_ingest
```

Tasks: [tasks/phase-5.md](tasks/phase-5.md)

---

## Phase 6 — Multi-wiki + SSE

**Goal:** One `wiki` process manages multiple repos. Remote agents work via SSE.

**Deliverables:**
- `registry.rs`:
  - Load `[[wikis]]` from `~/.wiki/config.toml`
  - `resolve(name) → WikiConfig` — default wiki if name omitted
- All CLI commands accept `--wiki <name>` flag
- `wiki search --all` — fan out tantivy across all registered wikis, merge + rank
- `wiki serve --sse :<port>` — SSE transport (multi-client, remote agents)
- All MCP tools accept optional `wiki` parameter

**Acceptance test:**
```bash
wiki --wiki work ingest analysis.json
wiki search --all "transformer scaling"   # hits both wikis
wiki serve --sse :8080                    # accepts remote connections
```

Tasks: [tasks/phase-6.md](tasks/phase-6.md)  |  Cross-cutting: [tasks/project.md](tasks/project.md)

---

## Phase 7 — Search Index — Incremental Update

**Goal:** The search index is no longer rebuilt on every `wiki search` call.
Build on first use, update incrementally after each ingest, explicit
`--rebuild-index` for fresh clones.

**Deliverables:**
- `search.rs`: `open_or_build_index`, `update_index(wiki_root, changed_slugs)`
- `integrate.rs`: all integrate functions call `update_index` after git commit
- `cli.rs`: `wiki search` uses `open_or_build_index` (not `build_index`)

**Acceptance test:**
```bash
wiki ingest paper.json          # index updated incrementally
wiki search "mixture of experts" # fast — no rebuild
wiki search --rebuild-index      # explicit full rebuild
```

Tasks: [tasks/phase-7.md](tasks/phase-7.md)

---

## Phase 8 — Repository Layout + Bundle Support

**Goal:** The wiki supports both flat pages and bundle folders (page + co-located
assets). Slug resolution handles both forms transparently. All walkers updated.

**Deliverables:**
- `markdown.rs`: `slug_for`, `resolve_slug`, `promote_to_bundle`, `is_bundle`
- `integrate.rs`: `write_asset_colocated`, `write_asset_shared`,
  `regenerate_assets_index`
- All walkers (`search.rs`, `graph.rs`, `context.rs`, `lint.rs`, `server.rs`)
  updated to use `slug_for` and `resolve_slug`
- `lint.rs`: orphan asset reference detection
- `cli.rs`: `wiki read <slug>` — fetch full content of one page
- MCP resources: bundle assets exposed at `wiki://{wiki}/{slug}/{filename}`

**Acceptance test:**
```bash
wiki ingest agent-skills/semantic-commit/ --prefix skills
# → skills/semantic-commit/index.md + lifecycle.yaml co-located
wiki read skills/semantic-commit
# → full page content
```

Tasks: [tasks/phase-8.md](tasks/phase-8.md)

---

## Phase 9 — Direct Ingest + Enrichment Contract

**Goal:** `wiki ingest <path>` works for files and folders without an LLM step.
`analysis.json` replaced by `enrichment.json` — frontmatter enrichment only.
`SuggestedPage` and `Action` removed.

**Deliverables:**
- `analysis.rs`: `Enrichment`, `QueryResult`, `Asset`, `AssetKind`,
  `ContentEncoding` structs; `Analysis` rebuilt; `SuggestedPage`/`Action`/
  `DocType`/`PageType` removed
- `ingest.rs`: `Input::Direct` (default) + `Input::AnalysisOnly` (legacy);
  `DirectIngestOptions`
- `integrate.rs`: `integrate_direct_file`, `integrate_direct_folder`,
  `integrate_enrichment`, `integrate_query_result`, `integrate_analysis`;
  old `integrate` removed
- `markdown.rs`: `generate_minimal_frontmatter`, `extract_h1`, `merge_enrichment`
- `cli.rs`: `wiki ingest <path>` as primary form; `--analysis`, `--dry-run` flags;
  `--analysis-only` legacy flag
- `server.rs`: new `wiki_ingest` tool (primary); `wiki_ingest_analysis` (legacy)

**Acceptance test:**
```bash
wiki ingest agent-skills/semantic-commit/ --prefix skills
# → pages + co-located assets, no LLM needed
wiki ingest --analysis-only enrichment.json
# → enrichments applied to existing pages, query results written
```

Tasks: [tasks/phase-9.md](tasks/phase-9.md)

---

## Phase 10 — Context Retrieval + wiki read + instruct update

**Goal:** `wiki context` returns ranked references (slug, URI, path, score) —
never page bodies. `wiki read` fetches a single page. `wiki instruct` gains
named topic variants covering doc authoring and the enrichment contract.

**Deliverables:**
- `context.rs`: `ContextRef` struct; `context` returns `Vec<ContextRef>`,
  body assembly removed
- `search.rs`: `score: f32` added to `SearchResult`
- `cli.rs`: `wiki context` prints reference list; `wiki read <slug>` (moved
  from Phase 8 stub to full implementation); `wiki instruct <topic>`
- `server.rs`: `wiki_context` returns `Vec<ContextRef>`; `wiki_read` tool;
  `wiki_instruct` gains `topic` param; prompts updated
- `src/instructions.md`: `## doc-authoring` and `## enrichment` sections added;
  `## ingest-workflow` updated; `suggested_pages` contract removed

**Acceptance test:**
```bash
wiki context "MoE scaling efficiency"
# → slug, uri, path, title, score per result — no page bodies
wiki read concepts/mixture-of-experts
# → full page content
wiki instruct doc-authoring
# → frontmatter schema + read_when discipline
wiki instruct enrichment
# → enrichment.json schema + field rules
```

Tasks: [tasks/phase-10.md](tasks/phase-10.md)

---

## Phase 11 — ACP Transport

**Goal:** `wiki serve --acp` works as a native Zed / VS Code agent. Sessions
are streaming and multi-turn. `src/instructions.md` injected at `initialize`.

**Deliverables:**
- `acp.rs`: `WikiAgent` implementing `Agent` trait from `agent-client-protocol`;
  `AcpSession`; workflow dispatch (`Ingest`, `Research`, `Lint`, `Enrichment`)
- `cli.rs`: `wiki serve --acp [--wiki <name>]`
- Zed config snippet in README
- New deps: `agent-client-protocol = "0.10"`, `agent-client-protocol-tokio = "0.1"`

**Acceptance test:**
```bash
# In ~/.config/zed/settings.json:
# { "agent_servers": { "llm-wiki": { "command": "wiki", "args": ["serve", "--acp"] } } }
# Open Zed agent panel, select llm-wiki:
# "ingest agent-skills/semantic-commit/"
# → streams tool_call events, done with ingest report
```

Tasks: [tasks/phase-11.md](tasks/phase-11.md)

---

## Dependency Table by Phase

| Crate           | First used  |
|-----------------|-------------|
| `serde_json`    | Phase 0     |
| `serde_yaml`    | Phase 0     |
| `toml`          | Phase 0     |
| `comrak`        | Phase 1     |
| `walkdir`       | Phase 1     |
| `git2`          | Phase 1     |
| `tantivy`       | Phase 2     |
| `petgraph`      | Phase 3     |
| `rmcp`          | Phase 4     |
| `clap`          | Phase 0     |
| `tokio`         | Phase 0     |
| `anyhow`        | Phase 0     |
| `base64`        | Phase 9     |
| `agent-client-protocol` | Phase 11 |
| `agent-client-protocol-tokio` | Phase 11 |

No LLM dependency in any phase.

---

## What Each Phase Unlocks

| After phase | You can…                                                              |
|-------------|-----------------------------------------------------------------------|
| 1           | Feed `analysis.json` from any LLM, pages (+ contradictions) on disk  |
| 2           | Ask "what do I know about X?" and get page context                   |
| 3           | Surface orphans, stubs, query and cluster contradiction pages         |
| 4           | Use the wiki from Claude Code with full MCP tool access               |
| 5           | `/llm-wiki:ingest` as a one-command slash workflow                    |
| 6           | Manage multiple knowledge bases, serve remote agents                  |
| 7           | Search index incremental update — no rebuild on every query          |
| 8           | Co-locate assets with pages, stable bundle slugs, `wiki read`         |
| 9           | Ingest any file or folder directly, enrich frontmatter without LLM   |
| 10          | `wiki context` returns references not bodies, `wiki instruct <topic>` |
| 11          | `wiki serve --acp` — native Zed / VS Code agent, streaming workflows  |
