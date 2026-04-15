# Multi-wiki — Developer Guide

Phase 6 adds a global wiki registry so that one `wiki` process can manage
multiple repositories.  Single-wiki setups are unaffected — the registry is
entirely opt-in.

---

## `~/.wiki/config.toml` schema

The global config is a TOML file with one `[[wikis]]` array:

```toml
[[wikis]]
name    = "work"
path    = "/Users/me/work-wiki"
default = true

[[wikis]]
name   = "research"
path   = "/Users/me/research-wiki"
remote = "git@github.com:me/research-wiki.git"   # optional, for documentation
```

Field reference:

| Field     | Type    | Required | Description |
|-----------|---------|----------|-------------|
| `name`    | string  | ✅       | Identifier used in `--wiki <name>` and MCP URIs |
| `path`    | path    | ✅       | Absolute path to the wiki root |
| `default` | bool    | —        | Exactly one wiki may be `default = true` |
| `remote`  | string  | —        | Optional git remote URL (informational) |

**Validation rules** (enforced by `WikiRegistry::load`):
- Exactly zero or one wiki may have `default = true`.
- Every `path` must exist on the filesystem; the wiki name is included in the
  error message if it does not.

---

## `--wiki <name>` global flag

```
wiki --wiki research search "attention mechanism"
wiki --wiki work ingest analysis.json
```

The flag is declared `global = true` on `Cli` and therefore works with every
subcommand:  `ingest`, `search`, `context`, `lint`, `list`, `contradict`,
`graph`, `diff`, and `serve`.

**Resolution order:**

1. Try to load `~/.wiki/config.toml`.
2. If the file exists, call `WikiRegistry::resolve(--wiki value)`:
   - `--wiki` omitted → returns the entry with `default = true`.
   - `--wiki <name>` → returns the entry whose `name` matches.
   - Unknown name → error that lists all available names.
3. If `~/.wiki/config.toml` does not exist → fall back to
   `WikiConfig { root: cwd, name: "wiki" }` (single-wiki mode, unchanged
   behaviour).

---

## `wiki init --register`

```
wiki init ~/work-wiki --register
```

After initialising the wiki, `--register` appends the new entry to
`~/.wiki/config.toml` (created if absent).  The first wiki registered
automatically becomes `default = true`.

---

## Cross-wiki search: `wiki search --all`

```
wiki search --all "reinforcement learning"
```

Fans out a tantivy query to every registered wiki, merges results by
descending BM25 score, and displays a table with a `WIKI` column.

The backing function is `search::search_all(registry, query, limit)` which
returns `Vec<SearchResultWithWiki>`.  Cross-wiki contradictions surface
naturally — if two wikis hold contradictory claims on the same topic, both
pages appear in the ranked list with their wiki labels.

---

## SSE setup — `wiki serve --sse :<port>`

```
wiki serve --sse :8080
```

Starts an HTTP server using rmcp's `SseServer` (axum-backed).

- `GET  /sse`              — SSE stream; server sends an `endpoint` event
                             pointing to the POST path for this session.
- `POST /message?sessionId=<id>` — JSON-RPC messages from the client.

Each connecting client receives an independent `WikiServer` instance.  There
is no shared mutable state between sessions.

**With a registry:**

```
wiki --wiki research serve --sse :8080
```

The registry is loaded once at startup.  Every new MCP session is given a
`WikiServer` that holds the registry, so all five tools (`wiki_ingest`,
`wiki_context`, `wiki_search`, `wiki_lint`, `wiki_list`) can target named
wikis via their `wiki` parameter.

**Graceful shutdown:**  Ctrl-C cancels the `CancellationToken` returned by
`SseServer::with_service`, which triggers axum's graceful shutdown.

**When to use SSE vs stdio:**

| | stdio | SSE |
|---|---|---|
| Single MCP client (Claude Code) | ✅ preferred | works |
| Multiple simultaneous clients | ✗ | ✅ preferred |
| Remote / containerised agents | awkward | ✅ preferred |
| Debugging (human-readable) | ✅ | curl-able |

---

## MCP multi-wiki wiring

All five MCP tools accept `wiki: Option<String>`.  When a `WikiServer` is
constructed with `new_with_registry(root, registry)`, tool calls resolve the
target root via `registry.resolve(wiki.as_deref())` before performing the
operation.

Resource URIs are namespaced by wiki name:

```
wiki://{wiki_name}/{type}/{slug}
```

- `list_resources`          — enumerates pages from all registered wikis.
- `list_resource_templates` — mounts one URI template per registered wiki.
- `read_resource`           — resolves the wiki from the URI host component.

Single-wiki servers (`WikiServer::new(root)`) continue to use the legacy
`wiki://default/…` URI scheme; the wiki name in the URI is ignored.

---

## Cross-wiki contradictions

`wiki search --all` surfaces cross-wiki contradictions naturally: if the same
slug appears in multiple wikis with different content, both result rows will
appear in the merged output.  Use the `WIKI` column to identify provenance.

For MCP clients, `wiki_search` with `all_wikis: true` returns a JSON array
that includes a `wiki_name` field on each result.
