---
name: llm-wiki
description: llm-wiki — git-backed wiki engine. Use when ingesting sources, creating pages, researching questions, enriching metadata, or running lint. Delegates to wiki instruct for dynamic instructions.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

# llm-wiki

A git-backed wiki engine. The wiki binary contains workflow instructions.
To get instructions for any operation:

```bash
wiki instruct <workflow>
```

Where `<workflow>` is one of: `help`, `init`, `new`, `ingest`, `research`,
`lint`, `crystallize`, `frontmatter`.

Run the appropriate instruct command, then follow the returned instructions
step by step.
