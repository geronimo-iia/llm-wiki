# wiki_lint — scalability parameters

Date: 2026-08-20
Status: Proposed

## Context

`wiki_lint` runs all rules against the full wiki and returns a single
`LintReport` with every finding. The tool schema exposes three parameters:
`rules` (comma-separated rule filter), `severity` (error/warning filter),
and `wiki` (target space).

At small wiki sizes (< 200 pages) this is adequate. At 1,000+ pages a full
run can return hundreds of findings in one JSON response, which exceeds
practical LLM context budgets and makes agent workflows that need to triage
or fix findings page-by-page unworkable.

The ARCH wiki instance (`/spaces/ARCH`) has 1,352 pages and is representative
of production scale.

## Problem

Four distinct failure modes at scale:

1. **Output size**: a full run on a 1,352-page wiki with `missing-fields` active
   can return 500+ findings in one response. An agent cannot process this in a
   single context window.

2. **No triage path**: there is no way to ask "how many findings exist per rule?"
   without receiving the full findings list. An agent must pay the full output
   cost before deciding which rules to investigate.

3. **No subtree scoping**: findings cannot be filtered to a slug prefix. An agent
   working on `nrg-architecture/studies/` receives findings for the entire wiki.

4. **No incremental processing**: once an agent has identified a rule to fix,
   it cannot retrieve findings in bounded batches. It must hold the full list
   in context or re-run the tool repeatedly.

## Decision

Add four parameters to `wiki_lint`:

### `summary: bool`

When `true`, return counts only — no `findings` array. Response shape:

```json
{
  "wiki": "ARCH",
  "total": 847,
  "errors": 12,
  "warnings": 835,
  "by_rule": {
    "orphan": 312,
    "broken-link": 8,
    "missing-fields": 527
  }
}
```

Implemented in `run_lint`: collect findings normally, then branch on
`summary` before serializing. `by_rule` is a `HashMap<&'static str, usize>`
built from the findings vec before it is dropped.

### `path_prefix: Option<String>`

Filter findings to slugs that start with the given prefix. Applied after
all rules run, before severity filter and sort. Example:
`path_prefix: "nrg-architecture/studies"` returns only findings whose
`slug` starts with that string.

Implemented as a single `findings.retain(|f| f.slug.starts_with(prefix))`
call in `run_lint`. No change to rule implementations.

### `page_size: Option<usize>` and `cursor: Option<usize>`

Paginate the findings list. `page_size` defaults to no limit (current
behaviour) when absent. `cursor` is a zero-based offset into the sorted
findings vec. Response gains two fields:

```json
{
  "next_cursor": 100,
  "has_more": true,
  ...
}
```

`next_cursor` is omitted (or `null`) when the last page has been reached.
Pagination is applied after all filters (`path_prefix`, `severity`) and
after the sort, so cursor values are stable within a single rule+prefix
scope as long as the wiki is not re-indexed between calls.

## Intended call sequence

The tool description must document the intended agent workflow explicitly:

> For large wikis: call with `summary: true` first to identify which rules
> have findings. Then narrow with `rules` (single rule) + `path_prefix`.
> Use `page_size` / `cursor` only when the scoped result is still large.

Without this guidance agents will call with no parameters, receive an
oversized response, and either truncate or loop expensively.

## Alternatives considered

**Streaming / server-sent events per finding**: rejected — MCP tool calls
are request/response; streaming would require a protocol change outside
this tool's scope.

**Persistent lint session with a cursor token**: rejected — stateful
sessions add server complexity and break the stateless MCP contract.
A numeric cursor over a deterministically sorted list is sufficient and
requires no server state.

**Pagination without prefix/summary**: rejected as insufficient. Paginating
800 findings without scoping first still produces an unworkable agent loop.
The four parameters are only useful in combination.

## Implementation notes

- `run_lint` signature gains `summary: bool`, `path_prefix: Option<&str>`,
  `page_size: Option<usize>`, `cursor: Option<usize>`.
- `LintReport` gains `next_cursor: Option<usize>`, `has_more: bool`,
  `by_rule: Option<HashMap<&'static str, usize>>` (present only when
  `summary: true`).
- `handle_lint` in `src/mcp/handlers.rs` extracts the four new args and
  passes them through.
- Tool schema in `src/mcp/tools.rs` adds four parameter entries and updates
  the description with the call sequence guidance.
- No change to any rule implementation (`rule_orphan`, `rule_broken_link`,
  etc.) — all filtering is post-collection in `run_lint`.
- Pagination cursor stability caveat: cursor values are only stable within
  a session where the index has not been rebuilt. This is acceptable — a
  re-index invalidates findings anyway.
