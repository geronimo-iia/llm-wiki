# MCP Server

`wiki serve` starts a [Model Context Protocol](https://modelcontextprotocol.io)
server that exposes the wiki engine to any MCP-capable LLM host.

---

## Transport

Phase 4 uses **stdio** transport only.  Pass `--sse <addr>` to get a warning;
full SSE support is Phase 6.

```bash
wiki serve          # stdio (default, use with Claude Code)
wiki serve --sse :8080  # ⚠ not yet implemented — falls back to stdio
```

---

## Tools

All tools accept an optional `wiki` parameter reserved for Phase 6 multi-wiki
routing.  In Phase 4 the parameter is silently ignored.

### `wiki_ingest`

```
wiki_ingest(analysis: object, wiki?: string) -> string
```

Deserialises the `analysis` JSON object into an `Analysis` struct, calls
`integrate::integrate`, and commits via `git::commit`.  Returns a human-readable
summary string on success, or an error string on failure (does **not** throw an
MCP error for analysis validation failures — the error is surfaced in the tool
result so the calling LLM can correct its output).

### `wiki_context`

```
wiki_context(question: string, wiki?: string, top_k?: integer) -> string
```

Calls `context::context(question, wiki_root, top_k)`.  Returns the top-K most
relevant wiki pages concatenated as Markdown.  Returns an empty string when no
pages match — never an error.

### `wiki_search`

```
wiki_search(query: string, wiki?: string, all_wikis?: boolean) -> JSON array
```

Calls `search::search(query, wiki_root, false)`.  Returns a JSON array of objects:

```json
[
  { "slug": "concepts/moe", "title": "Mixture of Experts",
    "snippet": "MoE routes tokens to expert subnetworks…",
    "score": 3.14, "page_type": "concept" }
]
```

`all_wikis` is reserved for Phase 6 and is currently a no-op.

### `wiki_lint`

```
wiki_lint(wiki?: string) -> JSON object
```

Calls `lint::lint(wiki_root)`.  Writes `LINT.md`, commits it, and returns a
JSON summary:

```json
{
  "orphan_count": 2,
  "orphans": ["concepts/old-page"],
  "missing_stub_count": 1,
  "missing_stubs": ["concepts/referenced-but-absent"],
  "active_contradiction_count": 1,
  "active_contradictions": ["contradictions/moe-scaling-efficiency"]
}
```

### `wiki_list`

```
wiki_list(wiki?: string, page_type?: string) -> JSON array
```

Lists all `.md` pages that have valid wiki frontmatter.  `page_type` accepts
`concept`, `source` (or `source-summary`), `contradiction`, `query` (or
`query-result`).  Returns a JSON array of objects:

```json
[
  { "slug": "concepts/moe", "title": "Mixture of Experts", "page_type": "concept" }
]
```

---

## Resources

Pages are exposed using the URI scheme `wiki://default/{type}/{slug}`.

**Supported type prefixes:**

| Prefix | Directory |
|---|---|
| `concepts/` | `concepts/` |
| `sources/` | `sources/` |
| `contradictions/` | `contradictions/` |
| `queries/` | `queries/` |

### Resource template

```
wiki://default/{type}/{slug}
```

### List resources

`list_resources` returns all pages as `wiki://default/{slug}` URIs (the slug
already contains the type prefix, e.g. `concepts/moe`).

### Read resource

`read_resource(uri)` resolves the URI to `{wiki_root}/{slug}.md` and returns the
raw Markdown content with MIME type `text/markdown`.

Unknown type prefixes or missing files return a resource-not-found error.

---

## Prompts

Prompts provide canned workflow instructions.  All prompts return a single `user`
role `PromptMessage` with a formatted multi-step instruction string.

| Name | Arguments | Purpose |
|---|---|---|
| `ingest_source` | `source?: string` | Step-by-step ingest workflow |
| `research_question` | `question?: string`, `save?: boolean` | Retrieve context + answer |
| `lint_and_enrich` | — | Run lint and address each finding |
| `analyse_contradiction` | `slug?: string` | Deep-dive into a contradiction |

---

## Server info

The server announces:

- **Instructions** — full contents of `src/instructions.md`, injected into every
  MCP session.
- **Capabilities** — `tools`, `resources`, `prompts`.

---

## Signals

`wiki serve` exits cleanly on `SIGINT` (Ctrl-C) via `tokio::signal::ctrl_c()`.

---

## Implementation notes

- Tools call library functions directly — no subprocess.
- The `WikiServer` struct is `Clone + Send + Sync + 'static` as required by rmcp.
- `get_peer` / `set_peer` store the `Peer<RoleServer>` handle in an
  `Arc<Mutex<Option<...>>>` for future resource-change notifications.
- `#[tool(tool_box)]` on `impl WikiServer { ... }` generates the static
  `ToolBox<WikiServer>`; `#[tool(tool_box)]` on `impl ServerHandler for WikiServer`
  generates the `list_tools` and `call_tool` delegation.
