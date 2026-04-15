# Claude Code Plugin

`llm-wiki` ships as a `.claude-plugin/` directory that Claude Code recognises
as an installable plugin.

---

## Directory structure

```
.claude-plugin/
├── plugin.json               — manifest: name, version, commands array
├── marketplace.json          — marketplace metadata (owner, description)
├── .mcp.json                 — MCP server config injected on install
├── README.md                 — plugin-specific install and usage guide
├── commands/
│   ├── help.md               — /llm-wiki:help
│   ├── init.md               — /llm-wiki:init
│   ├── ingest.md             — /llm-wiki:ingest
│   ├── research.md           — /llm-wiki:research
│   ├── lint.md               — /llm-wiki:lint
│   └── contradiction.md      — /llm-wiki:contradiction
└── skills/
    └── llm-wiki/
        └── SKILL.md          — the shared skill invoked by every command
```

---

## How commands work

Each command file in `commands/` is a short Markdown prompt that tells Claude
to invoke the `llm-wiki` skill with the matching command name. The skill
(`skills/llm-wiki/SKILL.md`) then runs:

```bash
wiki instruct <command>
```

and follows the returned instructions step by step.

This three-layer indirection means:

```
/llm-wiki:ingest
  → commands/ingest.md   (tells Claude: invoke SKILL with "ingest")
    → skills/llm-wiki/SKILL.md   (runs: wiki instruct ingest)
      → src/instructions.md § ingest-workflow   (authoritative steps)
```

The key property: **instructions live in the binary, not the plugin files.**
Updating `src/instructions.md` updates what every installed plugin sees on
the next `wiki instruct` call — no plugin reinstall required.

---

## Versioning policy

| What changed | Bump |
|---|---|
| `src/instructions.md` workflow steps | No version bump needed — instructions are fetched live |
| New slash command or renamed command | Bump `plugin.json` version (minor) |
| Breaking change to `analysis.json` schema or MCP tool signatures | Bump `plugin.json` version (major) |
| `Cargo.toml` version | Always kept in sync with `plugin.json` version |

The `plugin.json` version must match `Cargo.toml` so that `claude plugin`
diagnostics show a consistent installed version.

---

## Adding a new slash command

1. Add a file `commands/<name>.md` with frontmatter `description:` and body
   that delegates to the skill.
2. Add a corresponding `## <name>-workflow` section to `src/instructions.md`.
3. Add the command entry to `plugin.json` `commands` array.
4. Add the test `wiki_instruct_<name>_contains_<sentinel>` to `tests/plugin.rs`.

No changes to the skill file or the binary dispatch are needed unless the new
command requires a new `wiki <subcommand>`.
