# Study: wiki_history — git log for a page

Add a `wiki_history` tool that returns the git commit history for a
specific page. Enables trust assessment ("is this stale?"), session
tracking ("what did I add last session?"), and change auditing.

## Current state

The engine uses git for commits (`wiki_content_commit`, auto-commit
on ingest) but exposes no history. The only timestamp is
`last_updated` in frontmatter — manually maintained, often stale.

## Proposed behavior

### CLI

```
llm-wiki history <slug|uri>
            [--limit <n>]           # default: 10
            [--format <fmt>]        # text | json
            [--wiki <name>]
```

### MCP

```json
{
  "slug": "concepts/moe",
  "limit": 10
}
```

### Response

```json
[
  {
    "hash": "a3f9c12",
    "date": "2025-07-21T14:32:01Z",
    "message": "ingest: concepts/moe.md",
    "author": "Jerome Guibert"
  }
]
```

## Implementation

Use `git log --follow --format=... -- <path>` on the resolved file
path. `--follow` tracks renames (flat→bundle migration).

For bundles, log the `index.md` file. For sections, log the
directory.

## Interaction with existing features

- `wiki_diff` (future) would show the actual content changes between
  two commits from this history
- Bootstrap could check recent history to report activity
- Crystallize could reference the last commit to avoid duplicating
  recent work

## Open questions

- Should history include commits from before the page was ingested
  (e.g. manual git commits)?
- Should `--follow` be default or opt-in? Renames are rare but
  `--follow` has a performance cost on large repos.

## Tasks

- [ ] Spec: `docs/specifications/tools/history.md`
- [ ] `src/ops/history.rs` — git log wrapper
- [ ] `src/mcp/tools.rs` — add `wiki_history` tool
- [ ] `src/mcp/handlers.rs` — handler
- [ ] `src/cli.rs` — `History` command
- [ ] `src/main.rs` — CLI output
- [ ] Tests
- [ ] Decision record, changelog, roadmap, skills
