---
title: "Lint"
summary: "How to use wiki_lint to catch broken links, orphans, missing fields, stale pages, and unknown types."
---

# Lint

`wiki_lint` runs deterministic, index-based rules against your wiki. All
checks are pure index queries — no file I/O, no LLM involvement — so it
is fast and safe to run in CI, after ingest, or as the first step in a
review workflow.

## When to run it

- **After ingest** — catch broken links and missing fields before they
  accumulate
- **Before commit** — verify the wiki is clean as a local gate
- **In CI** — `llm-wiki lint --severity error` exits non-zero on any error
- **In crystallize** — run at the end of an extraction session to surface
  issues introduced by new pages

## The 8 rules

| Rule ID | Severity | What it catches |
|---------|----------|-----------------|
| `broken-link` | error | A slug in `body_links` or frontmatter edge fields (`sources`, `concepts`, `superseded_by`) does not exist in the index |
| `missing-fields` | error | Required frontmatter fields (per type schema) are absent |
| `unknown-type` | error | The `type` field value is not registered in the type registry |
| `orphan` | warning | Page has no incoming links and is not a section page |
| `stale` | warning | `last_updated` older than threshold **and** `confidence` below threshold |
| `articulation-point` | warning | Page whose removal disconnects the graph — add link paths that bypass this page |
| `bridge` | warning | Link whose removal disconnects the graph — add a parallel path between the two components |
| `periphery` | warning | Most structurally isolated page (eccentricity = diameter); skipped above `graph.max_nodes_for_diameter` |

Errors block CI. Warnings are informational — fix them when reviewing
wiki quality.

## Reading a finding

```json
{
  "slug": "concepts/moe",
  "rule": "broken-link",
  "severity": "error",
  "message": "broken link in body_links: concepts/ghost",
  "path": "/home/user/wikis/research/wiki/concepts/moe.md"
}
```

| Field | Meaning |
|-------|---------|
| `slug` | The page that has the problem |
| `rule` | Which rule fired |
| `severity` | `error` or `warning` |
| `message` | What was found and where |
| `path` | Absolute filesystem path of the offending file — use for direct `Edit` without a follow-up resolve call |

The full response includes a summary header:

```json
{
  "wiki": "research",
  "total": 3,
  "errors": 2,
  "warnings": 1,
  "findings": [...]
}
```

An empty `findings` array means the wiki is clean.

## How to act on each rule

### `broken-link`

A page references a slug that does not exist in the index.

Fix: either correct the slug in the page body or frontmatter, or create
the missing page. Remove the reference if the target is permanently gone.

### `missing-fields`

A required frontmatter field is absent for the page's type.

Fix: open the page and add the missing field. Use
`wiki_content_read(slug: "...", backlinks: false)` to inspect the current
frontmatter, then `wiki_content_write` to update it.

### `unknown-type`

The `type:` value in frontmatter is not registered.

Fix: correct the typo in the `type:` field, or register the new type with
`wiki_schema add`. Run `wiki_schema list` to see registered types.

### `orphan`

No other page links to this page.

Fix: add a `[[slug]]` reference from a relevant page, or add the slug to
a section's `concepts` or `sources` list. If the page is intentionally
standalone, suppress by linking from an index or root page.

### `stale`

The page is old (`last_updated` older than `stale_days`) **and**
low-confidence (`confidence` below `stale_confidence_threshold`).

Fix: review the page and update its content, then raise `last_updated`
and adjust `confidence` to reflect your current certainty. A page that
is old but `confidence: 0.9` is **not** flagged as stale.

### `articulation-point`

This page is a cut vertex — removing it splits the graph into two or more disconnected
components. It carries an outsized connectivity burden.

Fix: add at least one alternative link path that bypasses this page. Connecting two of
its neighbours directly reduces the fragility. Use `wiki_suggest` on each neighbour to
find good candidates.

### `bridge`

This link is the only connection between two parts of the graph. Removing the source or
target page would disconnect the graph.

Fix: add at least one more link between the two connected components. Look at pages on
either side of the bridge and add cross-links.

### `periphery`

This page is maximally isolated — its eccentricity equals the graph diameter. It is the
furthest from all other pages.

Fix: link it to more central pages. Run `wiki_suggest(slug: "<slug>")` to find the
best connection candidates. The `center` field in `wiki_stats` lists the hub pages most
worth connecting to.

## Typical workflow

```
wiki_lint()                          → get all findings
wiki_lint(severity: "error")         → triage errors first
wiki_lint(rules: "broken-link")      → focus on one rule
```

1. Run `wiki_lint()` — review the summary (`errors`, `warnings`).
2. Fix all `error` findings first — they indicate broken references or
   invalid structure.
3. Review `warning` findings — orphans and stale pages are quality issues,
   not structural breaks.
4. Re-run to confirm clean.

## Running a subset of rules

```bash
# Errors only
llm-wiki lint --severity error

# Specific rules
llm-wiki lint --rules broken-link,orphan

# Target a specific wiki
llm-wiki lint --wiki research
```

## CI usage

```bash
# Exits non-zero if any error finding exists
llm-wiki lint --severity error && echo "wiki is clean"
```

Add to your CI pipeline after `llm-wiki index rebuild` to catch regressions
on every push.

## Tuning the stale rule

The `stale` rule uses two thresholds configurable in `config.toml` (global)
or `wiki.toml` (per-wiki):

```toml
[lint]
stale_days                 = 90    # days before a page is considered old
stale_confidence_threshold = 0.4   # confidence below this + old = stale
```

Examples:

- Set `stale_days = 180` for a slower-moving reference wiki where 90-day
  cadence is too aggressive.
- Set `stale_confidence_threshold = 0.6` to flag pages more aggressively —
  only high-confidence pages escape the stale check.
- Set `stale_confidence_threshold = 0.0` to disable the confidence gate —
  any page older than `stale_days` is flagged regardless of confidence.

Per-wiki `[lint]` replaces the global value entirely (not merged key-by-key).
See [configuration.md](configuration.md) for the full resolution order.
