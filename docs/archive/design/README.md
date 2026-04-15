# Design docs

Specifications and decisions that define what `llm-wiki` must do.
Read these before implementing a module or changing a contract.

---

## Current — read these

| Doc | What it covers |
|-----|----------------|
| [repository-layout.md](repository-layout.md) | Wiki root, ingest source root, flat vs bundle pages, default categories, slug resolution |
| [epistemic-model.md](epistemic-model.md) | Why the five default categories exist and what each one means |
| [ingest.md](ingest.md) | Three ingest modes — direct, direct + enrichment, analysis-only (legacy) |
| [asset-ingest.md](asset-ingest.md) | Asset handling — co-located vs shared, bundle promotion, assets/index.md |
| [page-content.md](page-content.md) | How integrate assembles pages — direct ingest frontmatter, enrichment merge rules, query result body |
| [context-retrieval.md](context-retrieval.md) | wiki context returns references not bodies; wiki read fetches one page |
| [design-evolution.md](design-evolution.md) | How the design shifted from analysis-as-primary to enrichment-as-optional |
| [acp-transport.md](acp-transport.md) | ACP agent transport — wiki serve --acp, WikiAgent, workflow dispatch |
| [claude-plugin.md](claude-plugin.md) | .claude-plugin/ structure, slash commands, SKILL.md delegation |

---

## Historical — context only

| Doc | Status |
|-----|--------|
| [design.md](design.md) | Original design. Several sections superseded — see deprecation header inside. Core insight (contradictions as knowledge, git as backend, no LLM in binary) remains valid. |
| [analysis.schema.json](analysis.schema.json) | Generated from old Analysis struct. Stale after Phase 8 — regenerate with `wiki schema > docs/design/analysis.schema.json` |

---

## Generating the schema

After Phase 8 is implemented:

```bash
wiki schema > docs/design/analysis.schema.json
```

The schema reflects the current `Analysis` struct: `enrichments[]`,
`query_results[]`, `contradictions[]`, `assets[]`.
