# Decision: Index-Backed Query Trait — Not Worth It

## Context

`search`, `list`, and `build_graph` share setup patterns (get searcher,
resolve field handles, build query, iterate docs). Design review flagged
as "Minor."

→ [analysis prompt](../prompts/index-query-pattern.md)

## Decision

**Leave as-is.** No trait, no shared struct.

## Rationale

- 3 consumers with completely different return types (`Vec<PageRef>`,
  `PageList`, `WikiGraph`) — a trait needs an associated type that
  adds ceremony without clarity
- The shared part is ~5 lines of `is.field("slug")` calls — not
  enough duplication to justify an abstraction
- Query building is different each time (BM25, filter-only, AllQuery)
- A `FieldHandles` struct saves those 5 lines but adds a type nobody
  else uses
- Revisit if a 4th consumer appears with the same pattern
