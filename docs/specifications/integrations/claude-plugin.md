---
title: "Claude Plugin"
summary: "How llm-wiki ships as a Claude Code plugin — plugin structure, slash commands, MCP wiring, and skill delegation."
read_when:
  - Implementing or updating the Claude Code plugin
  - Understanding how slash commands delegate to llm-wiki instruct
  - Configuring the MCP server for Claude Code
status: draft
last_updated: "2025-07-15"
---

# Claude Plugin

`llm-wiki` ships as a Claude Code plugin via a `.claude-plugin/` directory at
the repo root.

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
    │   ├── new.md                   # /llm-wiki:new
    │   ├── ingest.md                # /llm-wiki:ingest
    │   ├── research.md              # /llm-wiki:research
    │   ├── lint.md                  # /llm-wiki:lint
    │   └── commit.md                # /llm-wiki:commit
    ├── hooks/
    │   └── hooks.json               # (reserved — no hooks in v0.1)
    └── skills/
        └── llm-wiki/
            └── SKILL.md             # generic skill — delegates to llm-wiki instruct
```

---

## `plugin.json`

```json
{
  "name": "llm-wiki",
  "version": "0.1.0",
  "description": "Git-backed wiki engine — ingest structured knowledge, search it, enrich with LLM metadata.",
  "author": { "name": "geronimo-iia" },
  "license": "MIT OR Apache-2.0",
  "commands": [],
  "mcpServers": {
    "llm-wiki": {
      "command": "llm-wiki",
      "args": ["serve"]
    }
  },
  "keywords": ["wiki", "knowledge-base", "mcp", "git", "research"],
  "repository": "https://github.com/geronimo-iia/llm-wiki"
}
```

---

## `marketplace.json`

```json
{
  "name": "llm-wiki",
  "owner": { "name": "geronimo-iia", "url": "https://github.com/geronimo-iia" },
  "plugins": [
    {
      "name": "llm-wiki",
      "source": "./.claude-plugin",
      "description": "Git-backed wiki engine — ingest structured knowledge, search it, enrich with LLM metadata.",
      "version": "0.1.0",
      "author": { "name": "geronimo-iia" }
    }
  ]
}
```

---

## `.mcp.json`

`llm-wiki serve` mounts all registered wikis at startup — no `--wiki` flag needed.

```json
{
  "llm-wiki": {
    "command": "llm-wiki",
    "args": ["serve"]
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
Commands delegate to `SKILL.md` which calls `llm-wiki instruct <workflow>` —
the binary is the single source of truth for instructions.

### `/llm-wiki:help`

```markdown
---
description: Show available wiki tools, slash commands, and workflows
allowed-tools: Bash
---
Invoke the llm-wiki skill with the `help` command, then follow its instructions.
```

### `/llm-wiki:init`

```markdown
---
description: Initialize a new wiki repo and register it
allowed-tools: Bash, Read, Write, Edit
---
Invoke the llm-wiki skill with the `init` command, then follow its instructions.
```

### `/llm-wiki:new`

```markdown
---
description: Create a new page or section in the wiki
allowed-tools: Bash, Read, Write, Edit
---
Invoke the llm-wiki skill with the `new` command, then follow its instructions.
```

### `/llm-wiki:ingest`

```markdown
---
description: Validate and index a file or folder in the wiki
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---
Invoke the llm-wiki skill with the `ingest` command, then follow its instructions.
```

### `/llm-wiki:research`

```markdown
---
description: Answer a question using the wiki as context
allowed-tools: Bash, Read
---
Invoke the llm-wiki skill with the `research` command, then follow its instructions.
```

### `/llm-wiki:lint`

```markdown
---
description: Structural audit — orphans, missing stubs, empty sections
allowed-tools: Bash, Read, Write, Edit
---
Invoke the llm-wiki skill with the `lint` command, then follow its instructions.
```

### `/llm-wiki:commit`

```markdown
---
description: Commit pending changes to git
allowed-tools: Bash, Read
---
Invoke the llm-wiki skill with the `commit` command, then follow its instructions.
```

---

## `skills/llm-wiki/SKILL.md`

```markdown
---
name: llm-wiki
description: llm-wiki — git-backed wiki engine. Use when ingesting sources, creating pages, researching questions, enriching metadata, or running lint. Delegates to llm-wiki instruct for dynamic instructions.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

# llm-wiki

A git-backed wiki engine. The wiki binary contains workflow instructions.
To get instructions for any operation:

```bash
llm-wiki instruct <workflow>
```

Where `<workflow>` is one of: `help`, `init`, `new`, `ingest`, `research`,
`lint`, `crystallize`, `commit`, `frontmatter`.

Run the appropriate instruct command, then follow the returned instructions
step by step.
```

---

## `src/instructions.md` — Instruction Source

`llm-wiki instruct` and the MCP server `instructions` field both read from
`src/instructions.md` embedded at compile time. Plugin files stay thin and
stable — updating instructions means releasing a new binary, not updating
plugin files. See [instruct.md](instruct.md).

---

## Hooks (reserved)

`hooks/hooks.json` registers no hooks in v0.1. Future candidates:

| Hook | Potential use |
|------|---------------|
| `Stop` | Auto-save current conversation as a `query-result` page |
| `PreCompact` | Preserve important findings before context compression |
