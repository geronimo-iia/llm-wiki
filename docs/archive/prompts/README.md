# Implementation Prompts

One prompt per phase. Each prompt is self-contained — paste it to start
implementing that phase. No need to read the full task doc first.

| Prompt | Phase | Goal |
|--------|-------|------|
| [phase-7.md](phase-7.md) | 7 | Search index incremental update — no rebuild on every query |
| [phase-8.md](phase-8.md) | 8 | Repository layout + bundle support — flat and bundle pages |
| [phase-9.md](phase-9.md) | 9 | Direct ingest + enrichment contract — breaking change |
| [phase-10.md](phase-10.md) | 10 | Context retrieval + wiki read + instruct update |
| [phase-11.md](phase-11.md) | 11 | ACP transport — native Zed / VS Code agent |

## How to use

1. Complete the previous phase and verify `cargo test` passes
2. Open the prompt for the next phase
3. Paste it to your coding agent (Claude Code, etc.)
4. The prompt references the task doc and design docs for full detail

## Dependencies between phases

```
Phase 7 (index)
  └─ Phase 8 (bundles) — needs incremental index for bundle page indexing
       └─ Phase 9 (direct ingest) — needs bundle support for folder ingest
            └─ Phase 10 (context retrieval) — needs enrichment contract stable
                 └─ Phase 11 (ACP) — needs instructions.md stable from Phase 10
```
