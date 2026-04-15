# LLM Wiki — Implementation Decisions

from https://github.com/vanillaflava/llm-wiki-claude-skills

What this implementation adds beyond Karpathy's original pattern. These are decisions that emerged from sustained real use.

## Karpathy Pattern → This Implementation

| Aspect | Original pattern | This implementation |
|---|---|---|
| Raw sources | Immutable permanent store | Flat queue; move to ingested/ is the commit |
| State tracking | log.md records what's processed | Filesystem presence is the record; log is audit-only |
| Query compounding | "Good answers can be filed back" (tip) | /crystallize as first-class operation |
| Session bootstrap | Schema document (CLAUDE.md) | Wiki + domain home pages ARE the bootstrap |
| Configuration | Suggestions for structure | wiki-config.md read by all five skills |
| Source attribution | Not specified | `changes:` frontmatter traces pages to sources |

## Filesystem-as-Truth

The most consequential decision: log.md is never read to determine processing state. The filesystem is the truth.

- File in raw/ = unprocessed
- File in ingested/subdir/ = processed
- Re-ingestion (accidental or intentional) works fine — synthesis surfaces whether content is unchanged, updated, or contradictory

This eliminates an entire class of state synchronization bugs.

## Ingestion Classification

Sources are classified by content, not file type, into archival subdirs:

- clippings — web saves, browser clips
- documentation — product docs, API references
- papers — academic papers, research material
- articles — blog posts, news, long-form
- data — CSV, JSON, structured datasets
- notes — freeform drafts, quick captures
- assets — unreadable files (always created)

Key rule: "Subject over document type." An article titled "Building a Wiki System" belongs in AI Learning if that's what a reader gains — not in wiki infrastructure because the title says "wiki."

## Backlink Quality Gate

Not every connection deserves a link. The test: "Would a reader of that page benefit from knowing about this content in a normal reading context — not just because they share a keyword, but because one genuinely informs the other?"

Graph density is not the goal.

## Privacy Model

Three-layer boundary:

1. **MCP filesystem scope** — the actual privacy boundary (what the agent can access)
2. **wiki_root** — the working scope for all skills
3. **blacklist** — prevents wiki page CREATION only, does NOT prevent reading

The blacklist is not a privacy boundary. If you don't want the LLM to read something, it must be outside the MCP scope entirely.

## Lint as Read-Only Audit

wiki-lint never auto-fixes. It produces a dated report in archive/ covering:
- Broken wikilinks + future breakage warnings (raw/ links that will break after ingest)
- Orphan pages (no index entry, no inbound links)
- Stale index entries (referenced file doesn't exist)
- Missing connections (significant term overlap, no mutual links)
- Orphaned sources in ingested/ (processed but left no trace in wiki)
- Conceptual flags (contradictions with Overview.md)

## Cloud Sync Awareness

Zero-byte files in cloud-synced vaults may be placeholders for files not yet downloaded locally. Skills flag these rather than treating them as empty files.
