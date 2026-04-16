# Improvements

## Roadmap

### 2. ACP streaming

Implement streaming tool calls in ACP workflows (tasks D → A → B → C → E).

→ [acp-tasks.md](acp-tasks.md) · [ACP SDK reference](implementation/acp-sdk.md) · [ACP transport spec](specifications/integrations/acp-transport.md)

### 3. Distribution channels

Decide which channels to support. Chocolatey is too heavy to maintain —
drop it. Evaluate: cargo install, cargo-binstall, homebrew, asdf.

- [ ] Fix `docs/release.md` — currently a verbatim copy of agentctl's; still says "agentctl"
- [ ] Replace agentctl-specific post-release steps with llm-wiki equivalents
- [ ] Decide final channel list and document only those

---

## Other improvements

### config

Parameter global-only should not be updated locally.

### graph

`graph.type` — documented but not yet implemented in `set_global_config_value`.

### init wiki

- [ ] Generate a `.gitignore`
- [ ] Prepare a `ci.yaml` template

### Documentation

**Implementation docs** — architecture overview, config system, server internals, logging:
- [ ] Architecture overview (module map, data flow, key abstractions)
- [ ] Config system (two-level resolution, adding new keys, serde patterns)
- [ ] Server internals (MCP stdio/SSE, ACP, transport lifecycle)
- [ ] Logging (rotation, format, file vs stderr, serve mode)

**User-facing docs** — installation and integration guides:
- [ ] Installation guide (cargo install, pre-built binaries, platform notes)
- [ ] Windows installation and usage notes
- [ ] IDE integration (VS Code, Cursor, Windsurf — beyond MCP config snippets)
- [ ] CI/CD integration (using llm-wiki in automated pipelines)

**Publishing** — use the wiki as a content source for static sites:
- [ ] Hugo integration guide (wiki pages as Hugo content, frontmatter mapping, example site)
- [ ] GitHub Pages deployment (CI workflow to build and publish from wiki repo)
