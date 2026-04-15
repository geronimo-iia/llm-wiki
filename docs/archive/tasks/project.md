# Project Infrastructure Tasks

Cross-cutting tasks not tied to a single phase.
Each section notes when it should be done relative to the roadmap phases.

---

## README (`README.md`)

The README is updated incrementally — one section unlocked per phase.

### After Phase 1

- [ ] **Install** section — `cargo install llm-wiki`, minimum Rust version
- [ ] **Quick start** — `wiki init <path>`, then `wiki ingest analysis.json`
- [ ] **CLI reference** table — all Phase 1 commands with one-line descriptions
- [ ] **`analysis.json` contract** — minimal schema example, link to `docs/design/design.md`

### After Phase 2

- [ ] **CLI reference** — add `wiki search`, `wiki context`
- [ ] **Usage example** — end-to-end: external LLM produces analysis → ingest → context

### After Phase 3

- [ ] **CLI reference** — add `wiki lint`, `wiki contradict`, `wiki graph`, `wiki list`, `wiki diff`
- [ ] **Contradictions** section — brief explanation, link to design doc

### After Phase 4

- [ ] **MCP server** section:
  - `wiki serve` (stdio) and `wiki serve --sse :<port>`
  - Minimal `~/.claude/settings.json` snippet
  - Table of MCP tools with descriptions
  - MCP resources URI scheme (including `wiki://schema/analysis`)
- [ ] **CLI reference** — add `wiki schema`

### After Phase 5

- [ ] **Claude Code plugin** section:
  - `claude plugin add /path/to/llm-wiki` (local)
  - `claude plugin marketplace add geronimo-iia/llm-wiki` (marketplace)
  - Post-install: `/llm-wiki:init`
  - Table of slash commands

### After Phase 6

- [ ] **Multi-wiki** section — `~/.wiki/config.toml` example, `--wiki` flag, `--all` search
- [ ] **SSE** section — `wiki serve --sse`, remote agent connection

---

## Contributing (`CONTRIBUTING.md`)

Do before or during Phase 0.

- [x] **Prerequisites** — Rust stable (minimum version), `cargo`, `git`
- [x] **Build** — `cargo build`, `cargo check`
- [x] **Test** — `cargo test`, `cargo test --test integration_test`
- [x] **Lint** — `cargo clippy --deny warnings`, `cargo fmt --check`
- [x] **Run locally** — `cargo run -- ingest analysis.json`, `cargo run -- serve`
- [x] **Project structure** — brief module map, link to `docs/dev/architecture.md`
- [x] **Commit message format** — `<type>: <description>` (feat, fix, docs, test, refactor, chore)
- [x] **Branch naming** — `phase-<N>/<short-description>`
- [x] **PR process** — one phase per PR, CI must pass, changelog entry required
- [x] **Changelog** — all changes go in `CHANGELOG.md` under `[Unreleased]`; format follows Keep a Changelog
- [x] **Code style** — `rustfmt.toml` is authoritative; no manual formatting
- [x] **No LLM dependency** — PRs must not add `rig-core` or any LLM client crate

---

## Issue Templates (`.github/ISSUE_TEMPLATE/`)

Do before Phase 1 (before the first real release).

### Bug report

- [x] `.github/ISSUE_TEMPLATE/bug_report.md`:
  - `wiki --version` output
  - OS and Rust version
  - Command run (with sanitized paths)
  - Expected vs actual behaviour
  - `analysis.json` excerpt if ingest-related (no secrets)
  - Relevant log output (`RUST_LOG=debug wiki ...`)

### Feature request

- [x] `.github/ISSUE_TEMPLATE/feature_request.md`:
  - Which phase / component is affected
  - Problem it solves (not just the solution)
  - Proposed CLI / MCP tool signature if applicable
  - Alternatives considered

### Config file

- [x] `.github/ISSUE_TEMPLATE/config.yml`:
  - Disable blank issues
  - Link to `CONTRIBUTING.md` for questions
  - Link to `docs/design/` for architecture discussion

---

## Dependabot (`.github/dependabot.yml`)

Do during Phase 0 (CI setup).

- [x] Cargo ecosystem — weekly updates, `main` branch target
- [x] GitHub Actions ecosystem — weekly updates
- [x] Group patch updates together (reduce PR noise)
- [ ] Assign PRs to a reviewer

```yaml
version: 2
updates:
  - package-ecosystem: cargo
    directory: /
    schedule:
      interval: weekly
    groups:
      patch-updates:
        update-types: [patch]

  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

---

## `CHANGELOG.md` conventions

Establish format before Phase 0 so all phases follow it consistently.

- [x] Follow [Keep a Changelog](https://keepachangelog.com/) format
- [x] Sections: `Added`, `Changed`, `Fixed`, `Removed`
- [x] Every phase adds an entry under `[Unreleased]`
- [x] On release: move `[Unreleased]` to `[x.y.z] — YYYY-MM-DD`
- [x] First entry: `[0.1.0]` targeting Phase 1 completion (core write loop)

---

## Timing summary

| Task | When | Status |
|---|---|---|
| `CONTRIBUTING.md` | Phase 0 | done |
| Issue templates | Phase 0 | done |
| Dependabot config | Phase 0 | done |
| `CHANGELOG.md` conventions | Phase 0 | done |
| README: install + CLI | Phase 1 | |
| README: search + context | Phase 2 | |
| README: lint + graph | Phase 3 | |
| README: MCP server | Phase 4 | |
| README: Claude plugin | Phase 5 | |
| README: multi-wiki + SSE | Phase 6 | |
| README: layout + bundle | Phase 8 | |
| README: direct ingest + enrichment contract | Phase 9 | |
| README: context refs + wiki read + instruct topics | Phase 10 | |
| README: ACP transport + Zed integration | Phase 11 | |
