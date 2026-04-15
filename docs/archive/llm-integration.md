# LLM Integration Model

llm-wiki has no LLM dependency. The LLM is always external. The wiki provides
the infrastructure; the LLM provides the intelligence. Three integration
surfaces are available: MCP (tool-calling), ACP (session-oriented streaming),
and CLI (pipeline scripting).

---

## The LLM's Role

The LLM does three things in the wiki workflow:

1. **Enriches pages** — reads existing pages, produces claims, confidence,
   contradictions, and tags to merge into frontmatter
2. **Synthesizes answers** — reads context references, fetches relevant pages,
   produces answers that can be saved as query-result pages
3. **Enriches contradictions** — reads contradiction pages and their source
   pages, produces resolution analysis, re-ingests

The LLM does not author page bodies (except query results). It does not
manage files. It does not call git. The wiki engine handles all of that.

---

## MCP — Tool-Calling Integration

`wiki serve` starts an MCP server. Any MCP-compatible client (Claude Code,
other agents) can call wiki tools directly.

### Tools

| Tool | What it does |
|------|-------------|
| `wiki_ingest` | Ingest a file or folder, optionally with enrichment JSON |
| `wiki_ingest_analysis` | Apply enrichment JSON only (legacy) |
| `wiki_context` | Find relevant pages for a question — returns references |
| `wiki_read` | Fetch full content of one page by slug |
| `wiki_search` | Full-text search — returns ranked results |
| `wiki_lint` | Run structural audit — returns lint report |
| `wiki_list` | List pages, optionally filtered by type |
| `wiki_instruct` | Get usage instructions for a specific workflow |

### Prompts

Named workflow templates that guide the LLM through multi-step operations:

| Prompt | Workflow |
|--------|---------|
| `ingest_source` | Ingest a source, enrich it, detect contradictions |
| `research_question` | Find relevant pages, synthesize answer, optionally save |
| `lint_and_enrich` | Run lint, enrich active contradictions, re-ingest |
| `analyse_contradiction` | Deep analysis of a single contradiction page |

### Transport

```
wiki serve              # stdio — Claude Code, local agents
wiki serve --sse :8080  # SSE — remote agents, multi-client
```

---

## ACP — Session-Oriented Integration

`wiki serve --acp` starts an ACP agent. ACP is the protocol used by Zed's
agent panel and VS Code agent extensions. It is session-oriented and streaming
— every step of a workflow streams back as an event visible in the IDE.

```json
{
  "agent_servers": {
    "llm-wiki": {
      "command": "wiki",
      "args": ["serve", "--acp"]
    }
  }
}
```

On connection, the wiki injects `wiki instruct` as the system context — the
LLM starts every session already knowing the enrichment contract and doc
authoring rules.

Each prompt dispatches to a workflow (ingest, research, lint, enrichment)
and streams tool call events as it progresses. The user sees the workflow
unfold in real time.

---

## CLI — Pipeline Integration

For scripted workflows, the CLI pipes naturally:

```bash
# Find relevant pages, read the top result, feed to an LLM
wiki context "MoE scaling" --top-k 3
wiki read concepts/mixture-of-experts | my-llm enrich

# Ingest a folder, then enrich with LLM output
wiki ingest agent-skills/semantic-commit/ --prefix skills
my-llm analyze skills/semantic-commit/index.md | wiki ingest --analysis-only -
```

---

## The Enrichment Workflow

The recommended workflow for enriching existing pages:

```
1. wiki ingest <path>
   → pages exist, assets co-located, git committed

2. wiki context "<key concepts>"
   → ranked reference list

3. wiki read <slug> for each relevant page
   → full page content

4. LLM produces enrichment.json
   → enrichments[], query_results[], contradictions[]

5. wiki ingest --analysis enrichment.json
   → frontmatter enriched, contradictions written, git committed
```

Steps 2–5 are optional. A wiki of direct-ingested pages is already useful
for search and retrieval without any LLM enrichment.

---

## The Enrichment Contract

The LLM produces a JSON document with three arrays:

**`enrichments[]`** — one entry per page to enrich. Each entry targets an
existing page by slug and provides metadata to merge: claims, concepts, tags,
confidence, source references. The page body is never touched.

**`query_results[]`** — LLM-authored pages. Used when saving a synthesized
answer as a `queries/` page. The LLM writes the body; the wiki writes the
frontmatter.

**`contradictions[]`** — detected tensions between sources. The LLM must
call `wiki context` first to know what pages exist before writing contradictions.
Empty if `wiki context` was not called.

---

## The Research Workflow

```
1. wiki context "<question>"
   → ranked reference list (slug, uri, path, title, score)

2. wiki read <slug> for each relevant page
   → full content of selected pages

3. LLM synthesizes answer from page content
   → surfaces contradiction pages explicitly (they are context gold)

4. Optionally: save answer as query-result page
   → wiki ingest --analysis-only with query_results[] entry
```

---

## Instructions

`wiki instruct` prints usage instructions for a specific topic:

```
wiki instruct                  # full guide
wiki instruct doc-authoring    # frontmatter schema + read_when discipline
wiki instruct enrichment       # enrichment.json contract + field rules
wiki instruct ingest           # ingest workflow
wiki instruct research         # research workflow
wiki instruct lint             # lint workflow
wiki instruct contradiction    # contradiction enrichment workflow
```

These same instructions are injected at MCP connection time and at ACP
session initialization — the LLM always has the contract available without
a separate tool call.

---

## Multi-Wiki

All tools accept an optional `wiki` parameter to target a specific registered
wiki. Omit it to use the default wiki from `~/.wiki/config.toml`.

```
wiki_context(question: "MoE scaling", wiki: "research")
wiki_ingest(path: "...", wiki: "work")
wiki_search(query: "...", all_wikis: true)
```
