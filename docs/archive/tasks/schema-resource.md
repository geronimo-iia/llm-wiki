# Task — Schema Command + MCP Resource

Expose the enrichment JSON schema as a first-class CLI command and MCP resource.
The schema is generated on demand from the live Rust types via `schemars` — no
static file, no drift risk.

The schema reflects the final `Analysis` struct from Phase 8:
`enrichments[]`, `query_results[]`, `contradictions[]`, `assets[]`.
`SuggestedPage`, `Action`, `DocType`, `PageType` are removed.

`wiki schema` follows the same CLI ↔ MCP symmetry as all other commands:
every MCP tool or resource has a matching CLI entry point.

Depends on: Phase 8 (`analysis.rs` rebuilt with new struct).

---

## `src/schema.rs` (new)

- [ ] `pub fn enrichment_schema() -> String`
  — calls `schemars::schema_for!(Analysis)`, serialises to pretty-printed JSON
  — covers `Enrichment`, `QueryResult`, `Contradiction`, `Asset` and all nested types
- [ ] Re-export from `src/lib.rs`

## `cli.rs`

- [ ] Add `Schema` variant to the `Command` enum
  ```
  wiki schema       # print enrichment.json JSON Schema to stdout
  ```
- [ ] Handler: call `schema::enrichment_schema()`, print to stdout
- [ ] Document in `wiki --help` as: "Print the JSON Schema for enrichment.json"

## `server.rs`

- [ ] Register static resource `wiki://schema/enrichment`
  - MIME type: `application/schema+json`
- [ ] `read_resource` handler: match `wiki://schema/enrichment`
  → call `schema::enrichment_schema()`, return content
- [ ] No `notify_resource_updated` — schema only changes when binary is rebuilt
- [ ] Remove old `wiki://schema/analysis` resource if present from Phase 4

## Cargo.toml

- [ ] Add `schemars` dependency:
  ```toml
  schemars = { version = "0.8", features = ["preserve_order"] }
  ```
- [ ] Add `#[derive(JsonSchema)]` to `Analysis`, `Enrichment`, `QueryResult`,
  `Contradiction`, `Asset`, `Claim`, `Confidence`, `AssetKind`, `ContentEncoding`,
  `Dimension`, `Status` in `analysis.rs`

## Cleanup

- [ ] Remove `src/bin/gen_schema.rs` if present — superseded by `wiki schema`
- [ ] Update `docs/design/analysis.schema.json` — regenerate with:
  `wiki schema > docs/design/analysis.schema.json`
  (file name kept for compatibility; content now reflects enrichment contract)
- [ ] Update `docs/dev/architecture.md`: add `schema.rs` to module map
- [ ] Update `src/instructions.md` `## enrichment` section: note that
  `wiki schema` prints the full JSON Schema for validation

## Tests

- [ ] `wiki schema` output is valid JSON
- [ ] `wiki schema` output parses as JSON Schema (contains `"$schema"` field)
- [ ] Schema contains `enrichments`, `query_results`, `contradictions`, `assets`
  as top-level properties
- [ ] Schema does NOT contain `suggested_pages`, `doc_type`, `action`
- [ ] MCP resource `wiki://schema/enrichment` returns same content as `wiki schema`
