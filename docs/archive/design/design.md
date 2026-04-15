# `llm-wiki` — Rust + Git Wiki Engine Design

> **Historical document.** This was the original design specification.
> Several sections have been superseded by later design decisions.
> Read this for context and origin; use the documents below for current
> contracts.
>
> | Section | Superseded by |
> |---------|---------------|
> | The `analysis.json` Contract | [design-evolution.md](design-evolution.md) |
> | The Workflow | [design-evolution.md](design-evolution.md) |
> | Repository Layout | [repository-layout.md](repository-layout.md) |
> | CLI `wiki ingest` | [ingest.md](ingest.md) |
> | CLI `wiki context` | [context-retrieval.md](context-retrieval.md) |
> | MCP `wiki_ingest` tool | [ingest.md](ingest.md) |
> | MCP `wiki_context` tool | [context-retrieval.md](context-retrieval.md) |
> | `ingest_source` prompt | [design-evolution.md](design-evolution.md) |
>
> What remains valid: contradictions as knowledge, git as backend,
> no LLM inside the binary, tantivy search, MCP server structure.

## The Core Insight

Traditional RAG rediscovers knowledge **at query time** — every question starts from scratch.
Karpathy's LLM Wiki flips this: process sources **at ingest time**, building a persistent,
structured, cross-referenced knowledge base that grows smarter with every addition.

| Dimension              | Traditional RAG                     | LLM Wiki                              |
|------------------------|-------------------------------------|---------------------------------------|
| When knowledge is built| At query time (per question)        | At ingest time (once per source)      |
| Cross-references       | Discovered ad hoc or missed         | Pre-built and continuously maintained |
| Contradictions         | Often overlooked                    | **First-class knowledge nodes**       |
| Knowledge accumulation | None — stateless                    | Persistent, versioned, diff-able      |

**`llm-wiki` is the wiki engine — not the LLM.** It manages structured Markdown, git
history, full-text search, contradiction tracking, and MCP exposure. The LLM that reads
and analyzes documents is entirely external: it produces an `analysis.json` and hands
it to the wiki via `wiki ingest`.

For the rationale behind the five default categories (`concepts/`, `sources/`,
`contradictions/`, `queries/`, `raw/`), see [docs/design/epistemic-model.md](epistemic-model.md).

---

## Contradictions Are Knowledge, Not Errors

This is the key philosophical reframe. In the original proposal, lint passes detect
contradictions to flag them for cleanup. That framing is wrong.

Real knowledge domains are full of contradictions — they are *where the interesting
information lives*. A contradiction between two sources reveals:

- **Context-dependence** — claim A holds in domain X, claim B in domain Y
- **Time-dependence** — A was true in 2021, superseded by B in 2024
- **Scale-dependence** — A works at small scale, B at large scale
- **Methodology divergence** — different measurement approaches yield different results
- **Genuine open dispute** — the field hasn't resolved this yet

A contradiction page is *richer* than either source alone. It encodes the *structure*
of a knowledge domain — where boundaries are, what conditions change the answer,
what questions remain open.

### Contradiction Page Schema

```markdown
---
title: "MoE scaling efficiency: contradictory views"
type: contradiction
claim_a: "sparse MoE reduces effective compute 8x at same quality"
source_a: "sources/switch-transformer-2021.md"
claim_b: "MoE gains diminish sharply beyond 100B parameters"
source_b: "sources/moe-survey-2023.md"
dimension: context-dependent      # context | time | scale | methodology | open-dispute
resolution: >
  Both true: claim_a holds for pre-training FLOPs; claim_b applies to fine-tuning regime.
  The contradiction reveals a training-phase boundary.
epistemic_value: >
  Compute/quality tradeoff in MoE is phase-dependent — this is non-obvious from
  either paper alone. The contradiction is the finding.
status: resolved                   # resolved | active | under-analysis
tags: [moe, scaling, compute-efficiency]
related_concepts: ["mixture-of-experts.md", "scaling-laws.md"]
created: 2026-04-13
updated: 2026-04-13
---

## Claim A
...

## Claim B
...

## Analysis
...
```

Contradictions are never deleted. A "resolved" contradiction still carries the
analysis that explains *why* the two sources disagreed — that explanation is the
knowledge. `git log` preserves the full history of how the understanding evolved.

---

## The Workflow

```
External LLM                      wiki CLI
────────────────────────────────  ─────────────────────────────────────
Read source (PDF, URL, Markdown)
Analyze → produce analysis.json ─▶ wiki ingest analysis.json
                                   write pages + detect contradictions
                                   update tantivy + index.json
                                   git commit

wiki context "question"          ◀─ External LLM asks for context
  returns top-K relevant pages  ──▶ External LLM synthesizes answer
                                   (optionally: wiki ingest answer.json)

wiki lint                        ──▶ structural report: orphans, stubs,
                                     active contradiction pages
                                   External LLM enriches, re-ingests
```

1. **Obtain** — collect raw sources (papers, URLs, transcripts, Markdown files)
2. **Analyze** — **external LLM** reads the source and produces `analysis.json`
3. **Ingest** — `wiki ingest analysis.json` writes pages (and contradiction files
   if present), commits atomically
4. **Search / Context** — `wiki context "<question>"` returns relevant pages for
   an external LLM to synthesize answers from
5. **Lint** — `wiki lint` produces a structural report; external LLM enriches
   contradiction pages as needed, re-ingest the enriched analysis
6. **Save queries** — valuable Q&A sessions saved as `query-result` pages via ingest

---

## Why Git is the Perfect Backend

- The entire wiki is **plain Markdown files** — no proprietary format
- Every ingest session becomes a **commit** with a meaningful message
- `git diff HEAD~1` shows exactly what changed
- `git log --oneline wiki/contradictions/moe-scaling.md` traces how understanding evolved
- `git revert` rolls back a bad lint pass
- GitHub/Forgejo makes it **collaborative**

---

## The `analysis.json` Contract

This is the **primary interface** between the external LLM and the wiki.

### Two-step workflow

Contradiction detection requires knowing what pages already exist. The LLM must
call `wiki context` (or read MCP resources) **before** producing the analysis —
not after. The recommended sequence:

```
1. wiki_context(key concepts from source)  → read existing pages
2. Produce analysis.json (with contradictions informed by step 1)
3. wiki_ingest(analysis)                   → commit
```

If the LLM skips step 1, `contradictions[]` should be empty — the wiki will not
invent contradictions on its own.

### Schema

```json
{
  "source": "path/or/url/to/original",
  "doc_type": "research-paper | blog-post | transcript | thread | note | book-chapter",
  "title": "Switch Transformer: Scaling to Trillion Parameter Models",
  "language": "en",
  "claims": [
    {
      "text": "sparse MoE reduces effective compute 8x at same quality",
      "confidence": "high | medium | low",
      "section": "Results"
    }
  ],
  "concepts": ["mixture-of-experts", "scaling-laws", "transformer"],
  "key_quotes": ["..."],
  "data_gaps": ["evaluation only on pre-training, not fine-tuning"],
  "suggested_pages": [
    {
      "slug": "concepts/mixture-of-experts",
      "title": "Mixture of Experts",
      "type": "concept | source-summary | query-result",
      "action": "create | update | append",
      "tldr": "one-sentence summary",
      "body": "full Markdown body (without frontmatter — wiki adds that)",
      "tags": ["transformers", "scaling"],
      "read_when": ["Reasoning about MoE architecture or scaling efficiency"]
    }
  ],
  "contradictions": [
    {
      "title": "MoE scaling efficiency: contradictory views",
      "claim_a": "...",
      "source_a": "sources/switch-transformer-2021.md",
      "claim_b": "...",
      "source_b": "sources/moe-survey-2023.md",
      "dimension": "context | time | scale | methodology | open-dispute",
      "epistemic_value": "what this tension reveals that neither source captures alone",
      "status": "active | resolved | under-analysis",
      "resolution": "optional — only if status is resolved"
    }
  ]
}
```

### Key design decisions

**`slug` is required on `suggested_pages`.** The wiki uses it as the file path
(`{slug}.md`). Deriving a path from a title is ambiguous and fragile — the LLM
controls the canonical slug.

**`action` is required on `suggested_pages`.** Three values:
- `create` — new page; fail if slug already exists (prevents silent overwrites)
- `update` — replace the body of an existing page (frontmatter fields are merged)
- `append` — add a new section to an existing page's body

**`body` is plain Markdown without frontmatter.** The wiki generates frontmatter
from the other fields (`title`, `type`, `tags`, `tldr`, `read_when`, `slug`).
This avoids asking the LLM to correctly format YAML inside a JSON string.

**`contradictions[]` may be empty.** If the LLM did not call `wiki context` first,
it should omit contradictions rather than hallucinate them. The wiki validates but
does not generate contradictions itself.

**`doc_type` is an enum**, not a free string. The wiki uses it for display and
grouping; unknown values are rejected at ingest boundary.

### Validation

The wiki performs JSON Schema validation at the ingest boundary and returns clear
errors for:
- Unknown `doc_type` values
- `action: create` on an existing slug
- `action: update|append` on a missing slug
- Contradictions that reference non-existent `source_a` / `source_b` slugs

The external LLM is responsible for:
- Reading and understanding the source document
- Calling `wiki_context` before writing `contradictions[]`
- Setting the correct `action` for each `suggested_page`
- Writing `body` as valid Markdown

The wiki engine is responsible for:
- JSON Schema validation + clear error messages
- Writing `suggested_pages` as `{slug}.md` files with generated frontmatter
- Writing `contradictions[]` as `contradictions/*.md` files if present
- Git commit

---

## Frontmatter Schema

Every wiki page is a valid agent-foundation document — any agent in the system can
consume the wiki directly via the standard doc hub protocol.

```yaml
# ── Base (required) ────────────────────────────────────────────────────────────
title: "Mixture of Experts"
summary: "Sparse routing of tokens to expert subnetworks, trading compute for capacity."
read_when:
  - "Reasoning about MoE architecture or scaling efficiency"
  - "Comparing dense vs sparse transformer models"
status: active                   # active | deprecated | stub
last_updated: 2026-04-13

# ── Wiki extensions ────────────────────────────────────────────────────────────
type: concept                    # concept | source-summary | query-result | contradiction
tags: [transformers, scaling, efficiency]
sources: ["sources/switch-transformer-2021.md"]
confidence: high                 # high | medium | low
contradictions: ["contradictions/moe-scaling-efficiency.md"]
tldr: "MoE routes tokens to sparse expert subnetworks, trading compute for capacity."
```

---

## Repository Layout

```
wiki/                           # the git repo
├── raw/                        # original source files (never modified)
├── concepts/                   # concept pages
├── sources/                    # per-source summary pages
├── contradictions/             # contradiction nodes (written at ingest, surfaced in Phase 3)
├── queries/                    # saved Q&A results (tagged query-result)
├── LINT.md                     # ← committed every lint pass (Phase 3+)
└── .wiki/
    ├── config.toml             # doc-type rules, wiki metadata
    └── search-index/           # tantivy index — gitignored, rebuilt on demand
```

**The rule:** `wiki ingest` commits Markdown files atomically. The tantivy search
index is gitignored and rebuilt locally — it is not committed. A fresh clone runs
`wiki search --rebuild-index` to become fully functional.

---

## CLI Interface

```
wiki ingest <file|->            # integrate Analysis JSON (file or stdin)       [Phase 1]
wiki search "<term>"            # tantivy full-text search                        [Phase 2]
wiki context "<question>"       # return top-K relevant pages as Markdown context [Phase 2]
wiki lint                       # structural report: orphans, stubs, contradictions [Phase 3]
wiki list [--type concept|contradiction|query|source]                              [Phase 3]
wiki contradict                 # list contradiction pages, filter by status       [Phase 3]
wiki graph                      # emit DOT/mermaid concept graph                   [Phase 3]
wiki diff                       # show what last ingest changed (git diff wrapper) [Phase 3]
wiki serve [--sse :<port>]      # start MCP server (stdio default, SSE optional)  [Phase 4]
wiki instruct [<workflow>]      # print instructions for LLMs                     [Phase 4]
```

### Key design decisions

**`wiki ingest` is analysis-JSON-only.** There is no built-in "analyze this PDF"
pipeline. The split is clean: the wiki engine does not call any LLM.

```bash
# External LLM produces analysis, wiki integrates it
my-llm analyze paper.pdf | wiki ingest -
my-llm analyze https://... | wiki ingest -
wiki ingest analysis.json

# External LLM reads context, synthesizes answer
wiki context "how does MoE scaling work?" | my-llm answer
```

**`wiki context` is not `wiki query`.** It returns raw page content — ranked by
tantivy relevance — for the external LLM to synthesize from. No LLM call inside
the wiki binary.

**`wiki lint` is structural only.** It reports orphan pages, missing concept stubs,
and active contradiction pages. The external LLM reads the report and re-ingests
enriched analysis as needed.

---

## Rust Ecosystem

### No LLM dependency

`rig-core` is not a dependency. The wiki engine has zero LLM calls.

### Full-Text Search: `tantivy`

```toml
tantivy = "0.22"
```

Lucene-equivalent written in Rust. Powers `wiki search` and `wiki context`. Indexes
all Markdown content, frontmatter fields, tags. The index lives in
`.wiki/search-index/` (gitignored, rebuilt on demand from committed Markdown).

### Concept Graph: `petgraph`

```toml
petgraph = "0.6"
```

In-memory directed graph built at startup from frontmatter `[[links]]` and
`related_concepts` fields. Used by:
- Orphan detection (nodes with in-degree = 0)
- Contradiction cluster analysis
- `wiki graph` DOT/mermaid output

### Supporting Crates

| Crate               | Purpose                                      |
|---------------------|----------------------------------------------|
| `clap`              | CLI argument parsing                         |
| `serde_yaml`        | YAML frontmatter parse/write                 |
| `serde_json`        | Analysis JSON deserialize                    |
| `comrak`            | Full GFM Markdown parser (link extraction)   |
| `walkdir`           | Recursive directory traversal                |
| `git2`              | Libgit2 — commit, diff, log from code        |
| `toml`              | Config file parsing                          |
| `tokio`             | Async runtime                                |
| `anyhow`            | Error handling                               |
| `rmcp`              | MCP server (stdio + SSE)                     |

---

## Rust Module Architecture

```
src/
├── main.rs             # CLI entry point
├── cli.rs              # clap Command enum
├── server.rs           # rmcp WikiServer — tools + resources + prompts
├── ingest.rs           # deserialize Analysis JSON → integrate
├── integrate.rs        # write pages, write contradiction nodes, update indices
├── context.rs          # tantivy search → assemble Markdown context for LLM
├── lint.rs             # structural audit: orphans, stubs, active contradictions
├── search.rs           # tantivy index build + query
├── graph.rs            # petgraph build → DOT/mermaid output
├── contradiction.rs    # contradiction page read/write/cluster
├── git.rs              # commit, diff, log via git2
├── markdown.rs         # frontmatter parse/write (serde_yaml + comrak)
├── registry.rs         # multi-wiki registry — loads ~/.wiki/config.toml
└── config.rs           # per-wiki .wiki/config.toml
```

No `llm.rs`. No `embed.rs`. The binary has no LLM dependency.

---

## Ingest Pipeline

```
External LLM                wiki ingest
─────────────────────────   ──────────────────────────────────────────
Read source
Produce analysis.json   ──▶ 1. Deserialize + validate Analysis
                            2. Write suggested_pages → concepts/, sources/, queries/
                            3. Write contradictions/*.md if contradictions[] present
                            4. git commit -m "ingest: <title> — +N pages"
```

The tantivy index is not rebuilt at ingest time — `wiki search` rebuilds it on
demand (or `--rebuild-index`). No committed artefacts beyond Markdown files.

### `wiki lint` (structural only)

```
1. Walk all pages — find orphan nodes (petgraph in-degree = 0)
2. Find referenced-but-missing concept stubs
3. Walk contradictions/ — list status: active pages (unresolved)
4. Write LINT.md report
5. git commit -m "lint: <date> — M orphans, K stubs, N active contradictions"
```

The external LLM reads `LINT.md`, enriches contradiction pages, and re-ingests.

### `wiki context "<question>"`

```
1. Tantivy search → top-K relevant pages (ranked by BM25)
2. Include relevant contradiction pages (context gold)
3. Return formatted Markdown: page titles + full content
   (External LLM synthesizes the answer)
```

---

## MCP Server

The `wiki` binary runs in two modes: CLI (default) and MCP server (`wiki serve`).
This makes the entire wiki accessible to any MCP-compatible agent — Claude Code,
other agents in the system, or remote clients via SSE.

**Crate:** `rmcp` — official Rust MCP SDK (modelcontextprotocol/rust-sdk).
Uses `#[tool]` / `#[tool_router]` / `#[prompt]` macros.

### `wiki instruct`

Both CLI and MCP server expose instructions explaining how to use the wiki.
Source of truth: `src/instructions.md` — embedded at compile time.

```rust
#[tool_handler(
    name = "wiki",
    version = "0.1.0",
    instructions = include_str!("instructions.md")
)]
impl ServerHandler for WikiServer {}
```

**`src/instructions.md`** covers:
- The tool is an engine — bring your own LLM for analysis and synthesis
- The `analysis.json` contract: what to produce and how
- Workflow: `wiki_context` → LLM synthesizes → `wiki_ingest_analysis`
- Contradictions as knowledge: detect them in analysis, not as errors
- Multi-wiki: how to target a specific wiki, when to search `--all`

### MCP Tools

```rust
#[tool_router(server_handler)]
impl WikiServer {
    /// Integrate a pre-built Analysis JSON into the wiki
    #[tool(description = "Ingest an Analysis JSON document into the wiki")]
    async fn wiki_ingest(
        &self,
        #[tool(param)] analysis: serde_json::Value,  // analysis.json contents
        #[tool(param)] wiki: Option<String>,
    ) -> String { ... }

    /// Return top-K relevant pages as Markdown context for the calling LLM
    #[tool(description = "Return relevant wiki pages as context for a question")]
    async fn wiki_context(
        &self,
        #[tool(param)] question: String,
        #[tool(param)] wiki: Option<String>,
        #[tool(param)] top_k: Option<u32>,           // default: 5
    ) -> String { ... }                               // Markdown: page contents

    /// Full-text search
    #[tool(description = "Search the wiki for relevant pages")]
    async fn wiki_search(
        &self,
        #[tool(param)] query: String,
        #[tool(param)] wiki: Option<String>,
        #[tool(param)] all_wikis: Option<bool>,
    ) -> Vec<SearchResult> { ... }

    /// Structural audit: orphans, missing stubs, active contradictions
    #[tool(description = "Run a structural lint pass on the wiki")]
    async fn wiki_lint(
        &self,
        #[tool(param)] wiki: Option<String>,
    ) -> LintReport { ... }

    /// List pages by type
    #[tool(description = "List wiki pages, optionally filtered by type")]
    async fn wiki_list(
        &self,
        #[tool(param)] wiki: Option<String>,
        #[tool(param)] page_type: Option<String>,    // concept|contradiction|query|source
    ) -> Vec<PageSummary> { ... }
}
```

### MCP Prompts — named workflow templates

```rust
/// Ingest workflow: LLM reads source, produces analysis, wiki stores it
#[prompt(description = "Analyze a source and ingest it into the wiki")]
async fn ingest_source(
    &self,
    #[prompt(description = "Path or URL to the source document")] source: String,
) -> Vec<PromptMessage> {
    vec![PromptMessage::user(format!(
        "You are ingesting a source into the wiki. Steps:\n\
         1. Read the source at \"{source}\".\n\
         2. Call wiki_context with key concepts from the source to check for \
            existing pages and potential contradictions.\n\
         3. Produce an analysis.json following the contract in wiki instruct.\n\
         4. Call wiki_ingest with the analysis JSON.\n\
         5. Report which pages were created or updated, and any contradictions detected."
    ))]
}

/// Research workflow: LLM assembles context and synthesizes answer
#[prompt(description = "Answer a question using the wiki as context")]
async fn research_question(
    &self,
    #[prompt(description = "The question to answer")] question: String,
    #[prompt(description = "Save the answer as a wiki page?")] save: Option<bool>,
) -> Vec<PromptMessage> {
    let save_instruction = if save.unwrap_or(false) {
        "After answering, produce an analysis.json with type=query-result and \
         call wiki_ingest to save your answer as a wiki page."
    } else { "" };
    vec![PromptMessage::user(format!(
        "Call wiki_context(question: \"{question}\") to get relevant pages. \
         Synthesize a thorough answer using the returned context. \
         Surface any contradiction pages explicitly — they are context gold. \
         {save_instruction}"
    ))]
}

/// Lint + enrich workflow
#[prompt(description = "Run lint and enrich active contradictions")]
async fn lint_and_enrich(&self) -> Vec<PromptMessage> {
    vec![PromptMessage::user(
        "Call wiki_lint to get the structural report. \
         For each active contradiction: read both source pages via MCP resources, \
         analyse the dimension and epistemic value, produce an updated analysis.json \
         with the enriched contradiction, call wiki_ingest. \
         Report what was resolved vs what remains open.".into()
    )]
}
```

**Available prompts:**

| Prompt | Purpose |
|---|---|
| `ingest_source` | LLM reads source → produces analysis → wiki stores it |
| `research_question` | `wiki_context` → LLM synthesizes answer |
| `lint_and_enrich` | Lint report → LLM enriches contradictions → re-ingest |
| `analyse_contradiction` | Deep analysis of a single contradiction page |
| `cross_wiki_synthesis` | Synthesise across multiple wikis |

### MCP Resources

Wiki pages exposed as readable resources:

```
wiki://<wiki-name>/concepts/<slug>          → full page content
wiki://<wiki-name>/contradictions/<slug>    → contradiction page
wiki://<wiki-name>/sources/<slug>           → source summary
wiki://<wiki-name>/queries/<slug>           → saved Q&A
```

When a page is updated by `wiki_ingest`, the server emits
`notify_resource_updated(uri)` — clients automatically see fresh content.

### Transport modes

```bash
wiki serve              # stdio (default — Claude Code, local agents)
wiki serve --sse :8080  # SSE (remote agents, multi-client)
```

---

## Multi-Repository

A single `wiki` process manages multiple git repositories.

### Global config `~/.wiki/config.toml`

```toml
[[wikis]]
name    = "research"
path    = "/Users/geronimo/wikis/research"
default = true

[[wikis]]
name   = "work"
path   = "/Users/geronimo/wikis/work"
remote = "git@github.com:org/work-wiki.git"

[[wikis]]
name   = "sp-theory"
path   = "/Users/geronimo/build/sp_theory/agent-knowledge"
```

### CLI targeting

```bash
wiki search "mixture of experts"              # uses default wiki
wiki --wiki sp-theory search "SP80"           # target specific wiki
wiki --wiki work ingest analysis.json         # ingest into work wiki
wiki search --all "transformer scaling"       # cross-wiki search
```

### Cross-wiki search

`wiki search --all` fans out tantivy queries to all registered wikis and merges
results ranked by relevance. Contradictions across wikis are surfaced with their
source wiki label.

### MCP multi-wiki

All registered wikis mounted at startup. Resources namespaced by wiki name:

```
wiki://research/concepts/mixture-of-experts
wiki://work/concepts/transformer-scaling
```

---

## Binary Modes Summary

```
wiki <subcommand>           # CLI mode
wiki serve                  # MCP server, stdio transport
wiki serve --sse :8080      # MCP server, SSE transport (multi-client)
```

The same core library powers both modes. The MCP server is a thin `rmcp` wrapper
over the same functions the CLI calls.

---

## Implementation Phases

See [roadmap.md](roadmap.md) for the full phase breakdown. Summary:

| Phase | Goal | Key deliverable |
|---|---|---|
| 0 | Compile-green skeleton | Schema structs locked |
| 1 | Core write loop | `wiki ingest` → Markdown + git commit |
| 2 | Search + context | `wiki search` + `wiki context` |
| 3 | Graph + lint + contradiction surfacing | `wiki lint`, `wiki contradict`, `wiki graph` |
| 4 | MCP server | `wiki serve` + all tools + prompts |
| 5 | Claude plugin | `.claude-plugin/` complete + `wiki instruct` |
| 6 | Multi-wiki + SSE | `--wiki`, `--all`, `--sse` |

**Contradiction phasing:** `contradictions/*.md` files are written silently in
Phase 1 whenever `contradictions[]` is present in the analysis. The commands to
surface, query, and cluster them (`wiki contradict`, `wiki lint`, petgraph) land in Phase 3.

---

## Open Questions

1. **Merge strategy for concept pages** — when ingest produces updates to an existing
   concept: append new claims + mark previous section with `updated:` timestamp, let
   external LLM consolidate on next lint pass.

2. **Contradiction clustering** — petgraph can identify subtrees with high contradiction
   density, surfaced in lint report as a signal (not a cleanup task).

3. **gitignore policy** — `.wiki/search-index/` is gitignored (rebuilt on demand).
   Everything else — Markdown files, `LINT.md` — is committed. A fresh clone
   runs `wiki search --rebuild-index` to become fully functional.

4. **`analysis.json` validation** — the wiki should reject malformed analysis with
   clear error messages, since the external LLM is the author and may hallucinate
   field names. JSON Schema validation at ingest boundary.

5. **Asset ingest** — handling non-Markdown assets (images, YAML, scripts, data
   files) at ingest time. See [asset-ingest.md](asset-ingest.md).

6. **Ingest** — file/folder as the default entry point with optional analysis
   JSON enrichment; three modes: direct, direct + enrichment, analysis-only
   (legacy). See [ingest.md](ingest.md).

7. **Context retrieval** — `wiki context` always returns ranked references
   (slug, URI, path, title, score), never full page bodies. `wiki read` fetches
   a single page on demand. See [context-retrieval.md](context-retrieval.md).

8. **Design evolution** — how the accumulated decisions shift `analysis.json`
   from primary page-creation mechanism to frontmatter enrichment layer.
   Identifies what is superseded in this document. See
   [design-evolution.md](design-evolution.md).

9. **ACP transport** — `wiki serve --acp` as a native Zed / VS Code agent.
   Session-oriented, streaming, instructions injected at initialize. See
   [acp-transport.md](acp-transport.md).
