---
title: "Reject stable page id (ULID) feature"
summary: "Permanent page identity via ULID was evaluated and rejected — the lint loop already covers reorganization, and the strong guarantee requires complexity that is not warranted."
status: accepted
date: "2026-07-24"
---

# Reject stable page id (ULID) feature

## Decision

Do not implement stable page identity (`id:` frontmatter field, ULID-based
addressing, `duplicate-id`/`id-format` lint rules). The feature is not
scheduled for any future milestone.

## Context

como-technologies/llm-wiki implemented this across 4 commits (`238d131`→`1c8626f`).

The stated problem: slugs are unstable — a page move breaks every wikilink
pointing at the old path. A stable opaque id in frontmatter, resolved through
the Tantivy index, would let links survive reorganization.

## Rationale

**The problem is already solved by the lint loop.**

`wiki_lint --rules broken-link` finds every dangling link after a page move.
The LLM operating the wiki has full context: it knows what moved, it can read
which pages link to the old slug, and it can update them or take an explicit
decision. This is not a bug — it is the correct workflow for a knowledge base
where reorganization carries intent.

**The como implementation does not deliver the strong guarantee it implies.**

como stores the id→slug mapping exclusively in the Tantivy index — a derived,
non-committed, machine-local artifact. After a raw `git mv` + commit, the id
resolves to a stale path until `index rebuild` runs. During that window, an id
link fails in exactly the same way a slug link would. The "permanent" contract
is not upheld at the git layer.

The strong guarantee would require a committed artifact (`wiki/.id-map.toml` or
equivalent) — adding a new committed file type, potential merge conflicts, and
a maintenance surface. That complexity is not justified by the gain.

**Opt-in coverage is always partial.**

Ids are opt-in. Pages without an id get no benefit. In practice, adoption
would be uneven, creating a two-tier wiki where some links are "more stable"
than others with no clear rule for which pages deserve an id.

**git has no native file identity.**

Git is content-addressed. Files have no identity across renames — only content
hashes. An `id:` field in frontmatter is the only layer where stability can
live in a git-backed system, but it requires the engine to maintain the lookup.
This is an impedance mismatch with the architecture.

## Alternatives considered

- **Committed id-map file** (`wiki/.id-map.toml`): stronger guarantee, but
  adds a committed artifact, merge conflict risk, and a new invariant to
  maintain. Rejected as over-engineered for the actual use case.
- **Automatic redirect stubs on move**: would require tracking old slugs in
  committed state. Same complexity problem.

## como implementation summary

4 commits across 2 weeks (July 2026):

| Commit | Layer |
|--------|-------|
| `238d131` | Parse `id:` ULID from frontmatter; keyword-index it in Tantivy alongside `slug`/`uri`; warn on duplicate during rebuild |
| `e8446b7` | `EngineState::resolve_address`: slug-first, then ULID lookup via index; stale-index error if file missing |
| `6341193` | `duplicate-id` (error) + `id-format` (warning) lint rules; graph edges + backlinks resolve by id |
| `1c8626f` | `id` field in all outputs; `content new --id`; MCP `auto_id`/`id` params |

## Reference

como-technologies/llm-wiki commits `238d131`, `e8446b7`, `6341193`, `1c8626f`.
