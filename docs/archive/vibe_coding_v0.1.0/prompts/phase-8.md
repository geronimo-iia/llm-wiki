# Phase 8 — Claude Plugin

## Context

Phases 1–7 are complete. The binary is fully functional. You are now
making the Claude Code plugin installable and all slash commands working.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- File contents must match the spec exactly — no extra commands, no extra
  fields in JSON manifests.
- Do not modify any file under `docs/` or `src/`.
- `wiki instruct <workflow>` must already work before starting this phase.

## Specs to read before starting

Read this file in full before writing any code:

- `docs/specifications/integrations/claude-plugin.md`

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `.claude-plugin/plugin.json`

Update to match the spec in
`docs/specifications/integrations/claude-plugin.md` §plugin.json.

### 2. `.claude-plugin/marketplace.json`

Update to match the spec §marketplace.json.

### 3. `.claude-plugin/.mcp.json`

Update to match the spec §.mcp.json.

### 4. `.claude-plugin/commands/`

Write or update all 7 command files:
`help.md`, `init.md`, `new.md`, `ingest.md`, `research.md`,
`crystallize.md`, `lint.md`.

Each file delegates to `wiki instruct <workflow>`. Content pattern is
in `docs/specifications/integrations/claude-plugin.md` §Slash Commands.

### 5. `.claude-plugin/skills/llm-wiki/SKILL.md`

Update to match the spec §SKILL.md. Remove any contradiction workflow
references.

### 6. Verify `wiki instruct`

Confirm `wiki instruct <workflow>` returns correct step-by-step
instructions for all 7 workflows:
`help`, `new`, `ingest`, `research`, `lint`, `crystallize`, `frontmatter`.

## Exit criteria

Before marking Phase 8 complete:

- [ ] `claude plugin add /path/to/llm-wiki` succeeds
- [ ] `/llm-wiki:help` prints available tools and workflows
- [ ] `/llm-wiki:ingest` triggers the ingest workflow
- [ ] `/llm-wiki:crystallize` triggers the crystallize workflow
- [ ] `/llm-wiki:lint` triggers the lint workflow
