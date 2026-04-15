# LLM Wiki — Skill Architecture

from https://github.com/vanillaflava/llm-wiki-claude-skills

Five Claude agent skills that decompose the LLM wiki pattern into composable filesystem operations. Plain markdown instructions — no code, no backend, no dependencies beyond an LLM with filesystem access.

## The Five Skills

| Skill | Purpose | Requires |
|---|---|---|
| wiki-ingest | Process raw/ queue → synthesised wiki pages → move to ingested/ | read, write, move |
| wiki-query | Answer questions from compiled wiki with citations | read (write optional) |
| wiki-crystallize | Distil chat sessions into durable wiki pages | read, write |
| wiki-integrate | Weave a page into the knowledge graph (backlinks + index) | read, write |
| wiki-lint | Health-check: broken links, orphans, stale entries, contradictions | read (write to archive/) |

Minimal setup: ingest + query. Add the others as the wiki grows.

## Shared Configuration

All five skills read from a single `wiki-config.md` (YAML frontmatter):

- `wiki_root` — absolute path to wiki root
- `blacklist` — paths where wiki page creation is forbidden
- `index_excludes` — paths excluded from index.md (raw/, archive/, ingested/)
- `ingested_folder` + `ingested_subdirs` — archival taxonomy
- `log_format` — consistent across all skills

Each skill searches for this file on activation. If not found, runs an init flow that creates the config, index.md, log.md, and ingested/ subdirs.

## Filesystem as State Machine

The key architectural insight: the filesystem IS the state. No database, no embedding store, no external service.

- **raw/** is a queue — files enter here, exit via ingest
- **ingested/** is the commit log — presence of a file = processed
- **index.md** is the catalogue — the LLM's entry point for navigation
- **log.md** is audit-only — append-only, never read for state decisions
- **archive/** holds lint reports and deprecated pages

The move from raw/ to ingested/ is the atomic commit. If a move fails, the file stays in raw/ for retry.

## Skill Boundaries

Each skill has a clear, non-overlapping responsibility:

- **ingest** is the only skill that moves files out of raw/
- **integrate** is the only skill that adds backlinks to existing pages (ingest does this too during its flow, but integrate handles post-hoc linking)
- **lint** never modifies wiki content — report only
- **crystallize** is the only skill that captures ephemeral chat knowledge
- **query** is read-only unless the user explicitly asks to file an answer

## Source Traceability

Every wiki page created from a source includes a `changes:` frontmatter field:
```yaml
changes: Created by wiki-ingest from ingested/documentation/source-file.md
```

This is how lint confirms no source is orphaned — every file in ingested/ should have at least one wiki page referencing it.

## Key Design Choices

- Skills are plain instructional markdown, not code — portable across LLM platforms with minor adaptation
- `.skill` files are just zip archives containing SKILL.md
- Blacklist governs wiki page creation only, not reading — it is NOT a privacy boundary
- The MCP filesystem scope is the actual privacy boundary
- One source can produce multiple wiki pages
- Duplicate detection: before creating a new page, scan index.md for topic overlap
