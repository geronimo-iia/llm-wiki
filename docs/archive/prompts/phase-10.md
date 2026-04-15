# Phase 10 — Context Retrieval + wiki read + instruct update

You are implementing Phase 10 of llm-wiki. Phase 9 (direct ingest) is complete.

## Context

The full task list is in `docs/tasks/phase-10.md`.
Design refs: `docs/design/context-retrieval.md`, `docs/tasks/instruct-update.md`.

## The Goal

Three changes in this phase:

1. **`wiki context`** returns ranked references (slug, URI, path, title, score)
   — never page bodies. Breaking change.
2. **`wiki read`** fetches the full content of one page by slug (stub from
   Phase 8 becomes full implementation here).
3. **`wiki instruct <topic>`** prints a named section from `src/instructions.md`.
   `src/instructions.md` is rewritten to cover the enrichment contract and
   doc authoring rules.

## What to implement

### `src/context.rs` — return references not bodies

Replace the current `String` return with:

```rust
pub struct ContextRef {
    pub slug:  String,
    pub uri:   String,   // wiki://<wiki_name>/<slug>
    pub path:  String,   // absolute file path on disk
    pub title: String,
    pub score: f32,
}

pub fn context(wiki_root, wiki_name, question, top_k) -> Result<Vec<ContextRef>>
```

Remove all body assembly logic. Contradiction pages still included when relevant.
Empty result → empty vec, not error.

### `src/search.rs`

Add `score: f32` to `SearchResult`. Expose the tantivy BM25 score from the
collector.

### `src/cli.rs`

`wiki context "<question>"` prints one `ContextRef` per line:
```
slug:  concepts/mixture-of-experts
uri:   wiki://research/concepts/mixture-of-experts
path:  /Users/.../concepts/mixture-of-experts.md
title: Mixture of Experts
score: 0.94
```

`wiki read <slug> [--body-only]` — full implementation (was stub in Phase 8).

`wiki instruct [<topic>]` — extract named section from `src/instructions.md`:

```rust
fn extract_section(instructions: &str, topic: &str) -> Option<String> {
    let heading = format!("## {topic}");
    let start = instructions.find(&heading)?;
    let rest = &instructions[start..];
    let end = rest[heading.len()..]
        .find("\n## ")
        .map(|i| i + heading.len())
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}
```

Valid topics: `doc-authoring`, `enrichment`, `ingest`, `research`,
`contradiction`, `lint`. Unknown topic → error listing valid topics.

### `src/server.rs`

- `wiki_context` returns `Vec<ContextRef>` (breaking — was `String`)
- Add `wiki_read(slug, wiki, body_only) -> String`
- `wiki_instruct(topic: Option<String>) -> String`
- Update `research_question` prompt: `wiki_context` → `wiki_read` → synthesize
- Update `ingest_source` prompt: `wiki_context` → `wiki_read` → enrichment.json → `wiki_ingest`

### `src/instructions.md` — full rewrite

The file must have these section anchors (used by `extract_section`):

```
## doc-authoring
## enrichment
## ingest-workflow
## research-workflow
## contradiction-workflow
## lint-workflow
```

`## doc-authoring` must cover: frontmatter schema (all fields), `summary`
discipline (scope not title), `read_when` discipline (specific conditions,
2–4 entries), layout rules (flat vs bundle), what LLM must NOT write
(sources, contradictions, frontmatter block in body).

`## enrichment` must cover: enrichment.json schema (`enrichments[]`,
`query_results[]`, `contradictions[]`), field rules (union/append/set/preserve),
what not to include (no body in enrichments, no suggested_pages, no doc_type).

Remove all references to `suggested_pages`, `doc_type`, `action: create/update/append`.

## Tests

New `tests/context.rs`, extend `tests/mcp.rs`, `tests/search.rs`:

- `context` returns `Vec<ContextRef>` not `String`
- `ContextRef.uri` format correct
- `ContextRef.path` absolute and resolves to file
- `context` empty result → empty vec
- `SearchResult.score` positive, results ordered descending
- `wiki instruct` no args → full guide
- `wiki instruct doc-authoring` → only that section
- `wiki instruct unknown` → error with valid topics listed
- `extract_section` known/unknown/last-section cases
- MCP `wiki_context` → JSON array of ContextRef
- MCP `wiki_read` → page content string
- MCP `wiki_instruct(topic: "enrichment")` → section only
- Integration: `wiki context` output has slug/uri/path/title/score, no bodies
- Integration: `wiki read` resolves bundle slug

## Acceptance criteria

- `wiki context` never returns page bodies
- `wiki instruct doc-authoring` gives enough to write correct frontmatter
- `wiki instruct enrichment` gives enough to produce valid enrichment.json
- `src/instructions.md` has no `suggested_pages`, `doc_type`, `action: create`

## Constraints

- No LLM dependency
- `wiki_context` return type change is breaking — document in CHANGELOG
- Create `docs/dev/context.md` and `docs/dev/instruct.md`
