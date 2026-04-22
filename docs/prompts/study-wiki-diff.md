# Study: wiki_diff — changes between commits for a page

Show what changed in a page between two commits. Complements
`wiki_history` (which shows *when* things changed) with *what*
changed.

## Problem

The LLM can't answer "what changed in this page?" or "what did the
last ingest add?". `wiki_history` shows commit messages and dates
but not the actual content diff. The user must use `git diff`
manually.

## Proposed behavior

### CLI

```
llm-wiki diff <slug|uri>
          [--from <commit>]         # default: HEAD~1
          [--to <commit>]           # default: HEAD
          [--format <fmt>]          # text | json
          [--wiki <name>]
```

### MCP

```json
{
  "slug": "concepts/moe",
  "from": "HEAD~1",
  "to": "HEAD"
}
```

### Response (text)

Standard unified diff:

```diff
--- a/wiki/concepts/moe.md
+++ b/wiki/concepts/moe.md
@@ -5,6 +5,7 @@
 tags: [mixture-of-experts, scaling]
+sources: [sources/switch-transformer-2021]
 ---

 ## Overview
@@ -12,3 +13,7 @@
 MoE routes tokens to sparse expert subnetworks.
+
+## Key claims
+
+Switch routing achieves 4x speedup over dense baselines.
```

### Response (JSON)

```json
{
  "slug": "concepts/moe",
  "from": "a3f9c12",
  "to": "b7e4d56",
  "additions": 5,
  "deletions": 0,
  "diff": "--- a/wiki/concepts/moe.md\n+++ b/wiki/concepts/moe.md\n..."
}
```

## Implementation

Use `git diff <from> <to> -- <path>` on the resolved file path.
Same path resolution as `wiki_history` (flat file or bundle
`index.md`).

For bundles, diff the `index.md`. Asset changes could be listed
separately (added/removed files) but not diffed (binary).

## Shorthand commits

| Input | Meaning |
|-------|---------|
| `HEAD~1` | Previous commit |
| `HEAD` | Current commit |
| `a3f9c12` | Specific commit hash |
| omitted `--from` | `HEAD~1` |
| omitted `--to` | `HEAD` (working tree if uncommitted changes) |

When `--to` is omitted and there are uncommitted changes, show the
diff against the working tree (useful for reviewing before ingest).

## Interaction with existing features

- `wiki_history` — provides the commit hashes to use with `wiki_diff`
- Crystallize — "show me what I added this session" = diff from
  session start commit to HEAD
- Lint — could use diff to detect frontmatter regressions (dropped
  tags, lowered confidence)

## Open questions

- Should `wiki_diff` support diffing between two different pages
  (compare mode)? Probably not — that's a different tool.
- Should the diff include frontmatter changes separately from body
  changes? (Structured diff vs raw unified diff)
- Working tree diff (uncommitted changes) — include by default or
  require explicit flag?

## Tasks

- [ ] Spec: `docs/specifications/tools/diff.md`
- [ ] `src/ops/diff.rs` — git diff wrapper
- [ ] `src/mcp/tools.rs` — add `wiki_diff` tool
- [ ] `src/mcp/handlers.rs` — handler
- [ ] `src/cli.rs` — `Diff` command
- [ ] Tests
- [ ] Decision record, changelog, roadmap, skills
