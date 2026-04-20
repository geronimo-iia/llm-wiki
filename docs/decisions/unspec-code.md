# Decision: Code Without Corresponding Spec

## Context

Four modules exist without matching specification documents.

→ [analysis prompt](../prompts/unspec-code.md)

## Decision

| Module | Action | Rationale |
|--------|--------|-----------|
| `ops/logs.rs` + CLI `Logs` | Add to CLI spec | User-facing commands — users need to know they exist |
| `links.rs` | Add to page-content spec | `[[slug]]` extraction rules not formalized |
| `default_schemas.rs` | No spec needed | Embedding mechanism is internal; fallback behavior already in type-system.md |
| `space_builder.rs` | No spec needed | Orchestration detail; covered by `docs/implementation/index-schema-building.md` |
