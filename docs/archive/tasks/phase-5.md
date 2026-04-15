# Phase 5 — Claude Plugin

Goal: `.claude-plugin/` is complete and installable.
`/llm-wiki:<command>` slash commands work end-to-end via `wiki instruct`.

---

## `wiki instruct` completeness

- [ ] `wiki instruct help` — output lists all commands, all MCP tools, links to docs
- [ ] `wiki instruct init` — output: verify install, `wiki init <path>`, MCP config snippet
- [ ] `wiki instruct ingest` — output: two-step workflow (wiki_context → analysis.json → wiki_ingest), schema reminder
- [ ] `wiki instruct research` — output: wiki_context call, synthesis instructions, optional save
- [ ] `wiki instruct lint` — output: wiki_lint call, how to enrich contradictions, re-ingest loop
- [ ] `wiki instruct contradiction` — output: read page + source pages, dimension analysis, epistemic_value guidance, re-ingest

## `wiki init` command

- [ ] `wiki init <path>` — initialize a new wiki repo: `git init`, create directory structure (`concepts/`, `sources/`, `contradictions/`, `queries/`, `raw/`, `.wiki/config.toml`)
- [ ] `wiki init` with existing git repo → skip `git init`, create missing directories only
- [ ] Print post-init instructions (add MCP config, run `/llm-wiki:init`)

## `.claude-plugin/` files

- [ ] `plugin.json` — version matches `Cargo.toml`, all fields correct
- [ ] `marketplace.json` — owner URL correct, description matches README
- [ ] `.mcp.json` — `wiki serve` command path resolves after `cargo install`
- [ ] `README.md` — install steps verified working, post-install step accurate
- [ ] `commands/help.md` — delegates to SKILL, `wiki instruct help` output is useful
- [ ] `commands/init.md` — delegates to SKILL, `wiki instruct init` output covers MCP config
- [ ] `commands/ingest.md` — delegates to SKILL, `wiki instruct ingest` is actionable
- [ ] `commands/research.md` — delegates to SKILL, `wiki instruct research` is actionable
- [ ] `commands/lint.md` — delegates to SKILL, `wiki instruct lint` is actionable
- [ ] `commands/contradiction.md` — delegates to SKILL, `wiki instruct contradiction` is actionable
- [ ] `skills/llm-wiki/SKILL.md` — correct frontmatter, `wiki instruct <command>` instruction accurate

## Tests

**Test file:** `tests/plugin.rs`

### Unit tests

- [ ] `wiki instruct help` — output non-empty, contains all 6 command names
- [ ] `wiki instruct ingest` — output contains "analysis.json"
- [ ] `wiki instruct ingest` — output contains the two-step workflow sequence
- [ ] `wiki instruct research` — output contains "wiki_context"
- [ ] `wiki instruct lint` — output contains "LINT.md"
- [ ] `wiki instruct contradiction` — output contains "epistemic_value"
- [ ] `wiki init <tmp_path>` — creates `concepts/`, `sources/`, `contradictions/`, `queries/`, `raw/`, `.wiki/config.toml`
- [ ] `wiki init <existing_git_repo>` — no error, missing directories created

### Manual tests (document results)

- [ ] `claude plugin add /path/to/llm-wiki` — installs without error
- [ ] `/llm-wiki:help` in Claude Code — response is coherent and accurate
- [ ] `/llm-wiki:init` in Claude Code — LLM follows setup steps correctly
- [ ] `/llm-wiki:ingest` in Claude Code — LLM calls `wiki_context` then `wiki_ingest`
- [ ] `/llm-wiki:research` in Claude Code — LLM calls `wiki_context`, synthesizes answer
- [ ] `/llm-wiki:lint` in Claude Code — LLM calls `wiki_lint`, enriches a contradiction
- [ ] `/llm-wiki:contradiction` in Claude Code — LLM reads page, produces enriched analysis

## Changelog

- [ ] `CHANGELOG.md` — add Phase 5 section: Claude plugin, `wiki init`, `/llm-wiki:*` commands, `wiki instruct` completeness

## README

- [ ] **Claude Code plugin** section:
  - Local install: `claude plugin add /path/to/llm-wiki`
  - Marketplace install: `claude plugin marketplace add geronimo-iia/llm-wiki`
  - Post-install step: `/llm-wiki:init`
  - Slash commands table: command → description
- [ ] **`wiki init`** entry in CLI reference table

## Dev documentation

- [ ] `docs/dev/plugin.md` — plugin directory structure, how commands → SKILL → `wiki instruct` works, how to update instructions without changing plugin files
- [ ] `docs/dev/plugin.md` — versioning: when to bump `plugin.json` version vs `Cargo.toml` version
- [ ] Update `docs/dev/architecture.md` — mark Phase 5 complete
