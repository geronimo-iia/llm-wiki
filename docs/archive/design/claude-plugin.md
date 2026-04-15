# Claude Plugin Design

`llm-wiki` ships as a Claude Code plugin via a `.claude-plugin/` directory at the
repo root. This is the standard Claude Code plugin format — the same structure used
by tools in the Claude marketplace.

---

## Plugin Structure

```
llm-wiki/
└── .claude-plugin/
    ├── plugin.json                  # main manifest
    ├── marketplace.json             # marketplace listing
    ├── .mcp.json                    # MCP server config snippet
    ├── README.md                    # user-facing install + usage docs
    ├── commands/
    │   ├── help.md                  # /llm-wiki:help
    │   ├── init.md                  # /llm-wiki:init
    │   ├── ingest.md                # /llm-wiki:ingest
    │   ├── research.md              # /llm-wiki:research
    │   ├── lint.md                  # /llm-wiki:lint
    │   └── contradiction.md         # /llm-wiki:contradiction
    ├── hooks/
    │   └── hooks.json               # (reserved — no hooks in v0.1)
    └── skills/
        └── llm-wiki/
            └── SKILL.md             # generic skill — delegates to wiki instruct
```

---

## `plugin.json`

Main manifest. Declares the MCP server (`wiki serve`) and plugin metadata.

```json
{
  "name": "llm-wiki",
  "version": "0.1.0",
  "description": "Git-backed wiki engine — ingest structured knowledge, search it, surface contradictions.",
  "author": { "name": "geronimo-iia" },
  "license": "MIT OR Apache-2.0",
  "commands": [],
  "mcpServers": {
    "wiki": {
      "command": "wiki",
      "args": ["serve"]
    }
  },
  "keywords": ["wiki", "knowledge-base", "mcp", "git", "contradictions", "research"],
  "repository": "https://github.com/geronimo-iia/llm-wiki"
}
```

---

## `marketplace.json`

Enables `claude plugin marketplace add geronimo-iia/llm-wiki`.

```json
{
  "name": "llm-wiki",
  "owner": { "name": "geronimo-iia", "url": "https://github.com/geronimo-iia" },
  "plugins": [
    {
      "name": "llm-wiki",
      "source": "./.claude-plugin",
      "description": "Git-backed wiki engine — ingest structured knowledge, search it, surface contradictions.",
      "version": "0.1.0",
      "author": { "name": "geronimo-iia" }
    }
  ]
}
```

---

## `.mcp.json`

MCP server config snippet — used by `claude plugin install` to wire up the server.

```json
{
  "wiki": {
    "command": "wiki",
    "args": ["serve"]
  }
}
```

For a specific wiki (non-default):

```json
{
  "wiki": {
    "command": "wiki",
    "args": ["serve", "--wiki", "research"]
  }
}
```

---

## Installation

```bash
# From the Claude marketplace
claude plugin marketplace add geronimo-iia/llm-wiki
claude plugin install --scope user llm-wiki

# From a local clone
claude plugin add /path/to/llm-wiki

# After installing, complete setup
/llm-wiki:init
```

---

## Slash Commands

Each file in `commands/` becomes a `/llm-wiki:<name>` slash command.
Commands delegate to the generic skill (`SKILL.md`) which calls `wiki instruct`
to get dynamic instructions from the binary — the binary is the source of truth.

### `commands/help.md` → `/llm-wiki:help`

```markdown
---
description: Show available wiki tools, slash commands, and workflows
allowed-tools: Bash
---

Invoke the llm-wiki skill (using the Skill tool) with the `help` command,
then follow its instructions.
```

### `commands/init.md` → `/llm-wiki:init`

```markdown
---
description: Set up llm-wiki — verify install, initialize a wiki repo, configure MCP
allowed-tools: Bash, Read, Write, Edit
---

Invoke the llm-wiki skill (using the Skill tool) with the `init` command,
then follow its instructions.
```

### `commands/ingest.md` → `/llm-wiki:ingest`

```markdown
---
description: Analyze a source document and ingest it into the wiki
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

Invoke the llm-wiki skill (using the Skill tool) with the `ingest` command,
then follow its instructions.
```

### `commands/research.md` → `/llm-wiki:research`

```markdown
---
description: Answer a question using the wiki as context
allowed-tools: Bash, Read
---

Invoke the llm-wiki skill (using the Skill tool) with the `research` command,
then follow its instructions.
```

### `commands/lint.md` → `/llm-wiki:lint`

```markdown
---
description: Structural lint pass — orphans, stubs, active contradictions
allowed-tools: Bash, Read, Write, Edit
---

Invoke the llm-wiki skill (using the Skill tool) with the `lint` command,
then follow its instructions.
```

### `commands/contradiction.md` → `/llm-wiki:contradiction`

```markdown
---
description: Deep analysis of a contradiction page
allowed-tools: Bash, Read, Write, Edit
---

Invoke the llm-wiki skill (using the Skill tool) with the `contradiction` command,
then follow its instructions.
```

---

## `skills/llm-wiki/SKILL.md`

The generic skill. Commands delegate here; this calls `wiki instruct <command>` to
get the actual workflow from the binary. The binary is the single source of truth for
instructions — plugin files stay thin and stable across versions.

```markdown
---
name: llm-wiki
description: llm-wiki — git-backed wiki engine. Use when ingesting sources, researching questions, managing contradictions, or running lint. Delegates to wiki instruct for dynamic instructions.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

# llm-wiki

A git-backed wiki engine. The wiki binary contains workflow instructions.
To get instructions for any operation:

```bash
wiki instruct <command>
```

Where `<command>` is one of: `help`, `init`, `ingest`, `research`, `lint`, `contradiction`.

Run the appropriate instructions command, then follow the returned instructions step by step.
```

---

## `src/instructions.md` — the instruction source

`wiki instruct` (CLI) and the MCP server `instructions` field both read from
`src/instructions.md` embedded at compile time (`include_str!`). This means:

- Plugin commands stay as one-liners that call the skill
- The skill calls `wiki instruct <command>`
- The binary returns the actual workflow
- Updating instructions = releasing a new binary, not updating plugin files

---

## Hooks (reserved)

`hooks/hooks.json` exists but registers no hooks in v0.1. Future candidates:

| Hook | Potential use |
|---|---|
| `Stop` | Auto-save current conversation as a `query-result` page |
| `PreCompact` | Preserve important findings before context compression |

These would require the user to have a wiki initialized and a target wiki configured
— making them opt-in rather than default.
