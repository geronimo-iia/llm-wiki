# Phase 10 — Context Retrieval + wiki read + instruct update

Goal: `wiki context` returns ranked references (slug, URI, path, score) — never
page bodies. `wiki read` fetches a single page on demand. `wiki instruct` gains
named topic variants covering doc authoring and the enrichment contract.

Depends on: Phase 8 complete.
Design refs: [context-retrieval.md](../design/context-retrieval.md),
[instruct-update.md](instruct-update.md).

---

## `context.rs`

- [ ] `ContextRef` struct:
  ```rust
  pub struct ContextRef {
      pub slug:  String,
      pub uri:   String,   // wiki://<wiki-name>/<slug>
      pub path:  String,   // absolute file path on disk
      pub title: String,
      pub score: f32,
  }
  ```
- [ ] `context(wiki_root: &Path, wiki_name: &str, question: &str, top_k: usize) -> Result<Vec<ContextRef>>`
  — run BM25 search, map results to `ContextRef`
  — derive `uri` from wiki name + slug
  — derive `path` from wiki root + slug resolution
  — remove all body assembly logic
- [ ] Remove old `context` return type (`String`)

## `search.rs`

- [ ] Add `score: f32` field to `SearchResult`
- [ ] Expose score from tantivy collector in `search` function

## `cli.rs`

- [ ] `wiki context "<question>"` — print `ContextRef` list, one per line:
  ```
  slug: concepts/mixture-of-experts
  uri:  wiki://research/concepts/mixture-of-experts
  path: /Users/.../concepts/mixture-of-experts.md
  title: Mixture of Experts
  score: 0.94
  ```
- [ ] `wiki context "<question>" --top-k <n>` — limit results (default: 5)
  — `wiki read` added in Phase 8; no CLI changes here
- [ ] `wiki instruct` — print full `src/instructions.md` (existing behaviour)
- [ ] `wiki instruct <topic>` — print named section only
  — valid topics: `doc-authoring`, `enrichment`, `ingest`, `research`,
    `contradiction`, `lint`
  — unknown topic → error listing valid topics

## `server.rs`

- [ ] Update `wiki_context` tool — return `Vec<ContextRef>` instead of `String`
- [ ] Add `wiki_read` tool:
  ```rust
  async fn wiki_read(
      slug: String,
      wiki: Option<String>,
      body_only: Option<bool>,
  ) -> String
  ```
- [ ] Update `wiki_instruct` tool — add `topic: Option<String>` param
- [ ] Update `research_question` prompt — call `wiki_context` (returns refs),
  then `wiki_read` for each relevant ref, then synthesize
- [ ] Update `ingest_source` prompt — reflect new enrichment workflow:
  `wiki_context` → `wiki_read` → produce `enrichment.json` → `wiki_ingest`

## `src/instructions.md`

- [ ] Add `## doc-authoring` section — frontmatter schema, `summary` discipline,
  `read_when` discipline, layout rules (flat vs bundle), what LLM must not write
- [ ] Add `## enrichment` section — enrichment.json schema, field rules,
  what not to include, when to call `wiki_context` first
- [ ] Update `## ingest-workflow` — reflect new default:
  `wiki ingest <path>` first, `--analysis enrichment.json` optional
- [ ] Update `## Analysis JSON contract` — replace `suggested_pages` schema
  with `enrichments` + `query_results` + `contradictions`
- [ ] Remove all references to `suggested_pages`, `doc_type`, `action: create/update/append`
- [ ] Section anchors must match topic names exactly for extraction:
  `## doc-authoring`, `## enrichment`, `## ingest-workflow`,
  `## research-workflow`, `## contradiction-workflow`, `## lint-workflow`

## Section extraction logic

`wiki instruct <topic>` extracts a section from `instructions.md` by heading:

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

## Tests

**Test files:** `tests/search.rs` (extend), new `tests/context.rs`,
`tests/mcp.rs` (extend)

### Unit tests — `context.rs`

- [ ] `context` returns `Vec<ContextRef>`, not `String`
- [ ] `ContextRef.uri` format: `wiki://{wiki_name}/{slug}`
- [ ] `ContextRef.path` is absolute and resolves to existing file
- [ ] `context` with no matching pages → empty vec, no error
- [ ] `context` includes contradiction pages when relevant

### Unit tests — `search.rs`

- [ ] `SearchResult.score` is positive for matching results
- [ ] Results ordered by descending score

### Unit tests — `cli.rs` / `instruct`

- [ ] `wiki instruct` with no args → full guide returned
- [ ] `wiki instruct doc-authoring` → contains `## doc-authoring` content only
- [ ] `wiki instruct enrichment` → contains `## enrichment` content only
- [ ] `wiki instruct unknown` → error listing valid topics
- [ ] `extract_section` — known topic → correct section boundaries
- [ ] `extract_section` — last section (no trailing `##`) → returns to end of file
- [ ] `extract_section` — unknown topic → `None`

### Unit tests — `server.rs`

- [ ] `wiki_context` returns `Vec<ContextRef>` (not `String`)
- [ ] `wiki_read` returns page content for known slug
- [ ] `wiki_read` with unknown slug → error string, no panic
- [ ] `wiki_instruct(topic: None)` → full guide
- [ ] `wiki_instruct(topic: Some("doc-authoring"))` → section only
- [ ] `wiki_instruct(topic: Some("unknown"))` → error string

### Integration tests

- [ ] `wiki context "MoE scaling"` → output contains slug, uri, path, title, score
- [ ] `wiki read concepts/mixture-of-experts` → full page content printed
- [ ] `wiki read concepts/mixture-of-experts --body-only` → no frontmatter in output
- [ ] `wiki read` on bundle slug → resolves `index.md`, prints content
- [ ] `wiki instruct doc-authoring` → output contains `read_when` discipline
- [ ] `wiki instruct enrichment` → output contains `enrichments[]` schema
- [ ] MCP `wiki_context` over stdio → returns JSON array of `ContextRef`
- [ ] MCP `wiki_read` over stdio → returns page content string
- [ ] MCP `wiki_instruct` with `topic: "enrichment"` → returns section only

## Acceptance criteria

- [ ] An LLM calling `wiki instruct doc-authoring` can write correct frontmatter
  without reading any design doc
- [ ] An LLM calling `wiki instruct enrichment` can produce valid `enrichment.json`
  without reading `design-evolution.md`
- [ ] `wiki context` never returns page bodies — only references
- [ ] `wiki read` is the only way to get page content via CLI
- [ ] `src/instructions.md` contains no references to `suggested_pages`,
  `doc_type`, or `action: create/update/append`

## Changelog

- [ ] `CHANGELOG.md` — Phase 9: `wiki context` returns refs not bodies (breaking),
  `wiki read`, `wiki instruct <topic>`, `src/instructions.md` rewrite,
  `ContextRef` struct, `wiki_read` MCP tool

## README

- [ ] CLI reference — update `wiki context` description (returns refs not bodies)
- [ ] CLI reference — add `wiki read <slug>`
- [ ] CLI reference — update `wiki instruct` to show topic variants
- [ ] **Usage example** — update end-to-end flow: `wiki context` → `wiki read` →
  produce `enrichment.json` → `wiki ingest --analysis`

## Dev documentation

- [ ] `docs/dev/context.md` — `ContextRef` fields, URI scheme, why no bodies
- [ ] `docs/dev/instruct.md` — section extraction logic, how to add a new topic,
  authoring discipline for `src/instructions.md`
- [ ] Update `docs/dev/architecture.md` — mark Phase 9 complete, final module map
