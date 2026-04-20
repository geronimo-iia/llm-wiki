---
title: "Skills Architecture"
summary: "How LLM skill playbooks relate to llm-wiki — skills are external to the engine, live in the wiki repo, read by the IDE/plugin directly."
status: draft
last_updated: "2025-07-15"
---

# Skills Architecture

Skills are detailed LLM playbooks that guide an external LLM through
multi-step wiki workflows. **The engine does not manage skills.** Skills
are a separate concern — installed, read, and executed by the IDE, the
plugin system, or the user.

---

## 1. The Problem

The engine provides tools (`wiki_search`, `wiki_write`, `wiki_ingest`,
`wiki_commit`), but it cannot decide what to write, where to put it, or
how to synthesize knowledge. That's the LLM's job.

The LLM needs detailed instructions — not just tool descriptions, but
step-by-step workflows with decision logic, templates, rules, and edge
cases. These are skills.

The engine's `instructions.md` contains short workflow summaries (~10
lines each). The vanillaflava skills that inspired this project are 200+
lines each. That level of detail belongs outside the engine.

---

## 2. Separation of Concerns

| Concern | Who owns it |
|---------|------------|
| Tools (`wiki_search`, `wiki_write`, etc.) | Engine |
| Engine reference (commands, config, structure) | Engine (`llm-wiki instruct`) |
| Skill playbooks (ingest, crystallize, etc.) | External — IDE, plugin, user |
| Skill installation | User (clone a repo, copy files) |
| Skill reading | IDE/plugin (reads `skills/` directly) |
| Skill execution | IDE's LLM (uses engine tools) |

The engine never reads, resolves, or returns skill content. It provides
the tools. Skills tell the LLM how to use them.

---

## 3. Layout Convention

Skills live in `skills/` at the wiki repository root. One folder per
skill, each containing a `SKILL.md`:

```
my-wiki/
├── wiki.toml
├── schema.md
├── skills/                        ← skill playbooks (not managed by engine)
│   ├── ingest/
│   │   └── SKILL.md
│   ├── crystallize/
│   │   └── SKILL.md
│   ├── integrate/
│   │   └── SKILL.md
│   ├── new/
│   │   └── SKILL.md
│   └── research/
│       └── SKILL.md
├── inbox/
├── raw/
└── wiki/
```

### Why this structure

- **One folder per skill** — follows the Claude Code skill convention.
  Compatible with Claude plugin discovery.
- **`SKILL.md`** — standard name. Tools that understand skills look for
  this file.
- **Per-wiki** — a research wiki might have different ingest rules than a
  work wiki. Same reasoning as `schema.md`.
- **Versioned with the wiki** — committed to git, evolves with the wiki.
- **Not hidden** — `skills/` not `.skills/`. Visible, editable.
- **Not inside `wiki/`** — not indexed, not searchable, not subject to
  lint. Engine ignores it.
- **Not managed by the engine** — `llm-wiki init` may create the empty
  folder, but the engine never reads from it.

---

## 4. SKILL.md Format

Each skill follows the Claude Code SKILL.md convention:

```markdown
---
name: wiki-ingest
description: Process source files into synthesized wiki pages. Reads
  sources, writes wiki pages with frontmatter, validates via wiki_ingest,
  commits via wiki_commit.
---

# Wiki Ingest

[full skill content — workflow steps, decision logic, templates, rules]
```

### Frontmatter fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Skill identifier |
| `description` | yes | One-paragraph summary — when to use, what it does |

---

## 5. Skill Sources

The user installs skills from their preferred source:

```bash
# From a dedicated skills repository
git clone https://github.com/.../llm-wiki-skills my-wiki/skills

# From agent-foundation
cp -r /path/to/agent-foundation/skills/* my-wiki/skills/

# Or write their own
mkdir -p my-wiki/skills/ingest && vi my-wiki/skills/ingest/SKILL.md

# Or use a git submodule
cd my-wiki && git submodule add https://github.com/.../skills skills
```

No skill installed = no skill content. The engine still works — the LLM
just doesn't have detailed playbooks and relies on the short
`instructions.md` orientation.

---

## 6. How Each Surface Reads Skills

The engine is not involved. Each surface reads skills directly:

| Surface | How it reads skills |
|---------|-------------------|
| **Claude Code** | `.claude-plugin/skills/` points to `skills/`. SKILL.md is native to Claude. |
| **Cursor** | User adds `skills/` to context via `@folder`, or workspace rules reference skill files. |
| **VS Code** | Same — user or extension reads `skills/` from the workspace. |
| **Zed (ACP)** | The IDE reads `skills/` from the workspace. The ACP agent does not serve skills. |
| **CLI** | User reads `skills/<workflow>/SKILL.md` directly, or uses `cat`. |

---

## 7. What the Engine Provides

The engine provides a complete engine reference via `llm-wiki instruct`.
This is the LLM's documentation of the engine — not a skill playbook.

### `llm-wiki instruct` (no argument)

Prints the full engine reference. Embedded in the binary. Always available.

### `llm-wiki instruct <section>`

Prints a specific section of the engine reference:

| Section | What it covers |
|---------|---------------|
| `help` | All tools, all commands, all workflows |
| `ingest` | What `wiki_ingest` does, what it validates, the pipeline, `auto_commit` behavior |
| `new` | What `wiki_new_page`/`wiki_new_section` create, frontmatter scaffold, flat vs bundle |
| `frontmatter` | All required fields, type taxonomy, per-type templates, update rules, common mistakes |
| `research` | How search works, `wiki_search` + `wiki_read` pattern |
| `lint` | What lint checks, what `wiki_lint` returns, how to interpret the report |
| `crystallize` | What crystallize means, the concept, when to use it (not the full playbook) |
| `commit` | What `wiki_commit` does, slug resolution, `--all` vs slugs, default messages |
| `config` | All config keys, scopes (global vs per-wiki), resolution order |
| `structure` | Repository layout, slug resolution, flat vs bundle, sections, `wiki://` URIs |

### What `instruct` is and is not

**Is:** The engine's own documentation for the LLM. "Here are all the
tools, what each command does, what frontmatter fields are required, what
types exist, how pages are structured, how config works."

**Is not:** A skill playbook. `instruct` doesn't tell the LLM how to
orchestrate a multi-step workflow. That's what skills do.

Example — frontmatter:

```
llm-wiki instruct frontmatter

## frontmatter

### Required fields

| Field | Required | Description |
|-------|----------|-------------|
| title | yes | Display name |
| summary | yes | One-line scope description |
| read_when | yes | 2-5 retrieval conditions |
| status | yes | active, draft, stub, generated |
| last_updated | yes | ISO date |
| type | yes | concept, paper, article, ... |

### Type taxonomy

| Type | Role |
|------|------|
| concept | Synthesized knowledge, one concept per page |
| paper | Academic source |
| article | Editorial (blogs, news) |
| documentation | Reference (API docs, specs) |
| query-result | Saved Q&A, crystallized sessions |
| section | Section index page |
...

### Per-type templates

Concept:
  title, summary, read_when, status, type: concept,
  tags, sources, concepts, confidence

Source (paper, article, etc.):
  title, summary, read_when, status, type: paper,
  tags, concepts, confidence, claims
...

### Update rules

1. wiki_read(<slug>) before writing.
2. Preserve existing list values — never drop tags, sources, concepts.
3. Add new values to lists.
4. Update scalars only with clear reason.
```

Example — commit:

```
llm-wiki instruct commit

## commit

Commit pending changes to git.

- wiki_commit(slugs?, message?) — commit specific pages or all
- Slugs: flat page → single file, bundle/section → entire folder recursively
- No slugs → commit all pending changes
- Default message: "commit: <slug1>, <slug2>" or "commit: all"
- Use after wiki_ingest when auto_commit is off
- Use after wiki_new_page, wiki_new_section, wiki_lint
```

Example — structure:

```
llm-wiki instruct structure

## structure

Repository layout:
  wiki.toml, schema.md, skills/, inbox/, raw/, wiki/

Slug resolution:
  concepts/moe → concepts/moe.md (flat) or concepts/moe/index.md (bundle)

wiki:// URIs:
  wiki://research/concepts/moe — full URI
  wiki://concepts/moe — default wiki

Flat vs bundle:
  No assets → flat (.md file)
  Has assets → bundle (folder + index.md + assets)
```

### MCP `instructions` field

The embedded `instructions.md` is injected at MCP/ACP server connect.
The LLM gets the orientation section automatically. Individual sections
are available via `wiki_instruct("frontmatter")` or
`llm-wiki instruct frontmatter`.

---

## 8. Relationship to `schema.md`

`schema.md` defines the wiki's structure — categories, type conventions,
folder organization. Skills reference `schema.md` as context:

```
Step 1 — Read schema.md to understand this wiki's conventions.
```

`schema.md` is *what* the wiki looks like. Skills are *how* to work with
it. Both live in the wiki repo. Neither is managed by the engine beyond
initial creation.

---

## 9. Init

`llm-wiki init` does **not** create a `skills/` folder. The engine has no
knowledge of skills. The user creates the folder when they install skills
from their preferred source:

```bash
cd my-wiki
git clone https://github.com/.../llm-wiki-skills skills
```

---

## 10. Delivery Across Surfaces

All surfaces return the same `instruct` content from the same embedded
source. The engine is the single source of truth for engine documentation.

| Surface | How it delivers `instruct` content |
|---------|-----------------------------------|
| **CLI** | `llm-wiki instruct <section>` prints to stdout |
| **MCP** | `wiki_instruct("<section>")` tool returns the section text |
| **ACP** | Skill-delegated workflows stream the section as `AgentMessageChunk` |

### ACP workflow dispatch

Engine-executed workflows (research, lint) call engine functions and
stream results directly.

Skill-delegated workflows (ingest, crystallize, new, commit) stream the
corresponding `instruct` section:

```rust
async fn run_skill(&self, session_id: &SessionId, workflow: &str, target: &str) {
    let instructions = crate::cli::extract_workflow(INSTRUCTIONS, workflow)
        .unwrap_or_else(|| format!("No instructions for: {workflow}"));

    if !target.is_empty() {
        self.send_message(session_id, &format!("Target: {target}\n\n{instructions}")).await;
    } else {
        self.send_message(session_id, &instructions).await;
    }
}
```

The IDE's LLM receives the engine instructions. If skills are installed
in `skills/`, the IDE loads them separately as additional context. The
engine doesn't know or care.

---

## 11. MCP Prompts and Resources

MCP has two primitives that map naturally to skills:

**MCP Prompts** — the server declares named prompts with arguments. The
client discovers them and the LLM calls them on demand. Designed for
reusable prompt templates.

**MCP Resources** — the server exposes readable content at URIs. The
client can list and read them.

The engine can use both to expose content without *managing* skills:

### Engine `instruct` sections as MCP prompts

The engine always serves its own documentation as MCP prompts:

```
prompts:
  wiki-instruct-help         → instruct help section
  wiki-instruct-frontmatter  → instruct frontmatter section
  wiki-instruct-ingest       → instruct ingest section
  wiki-instruct-commit       → instruct commit section
  wiki-instruct-structure    → instruct structure section
  ...
```

These are always available — embedded in the binary.

### Skills on disk as MCP prompts (passthrough)

If `skills/` exists in the wiki repo, the engine scans it at startup and
exposes each `SKILL.md` as an additional MCP prompt:

```
prompts:
  wiki-skill-ingest       → skills/ingest/SKILL.md content
  wiki-skill-crystallize  → skills/crystallize/SKILL.md content
  wiki-skill-integrate    → skills/integrate/SKILL.md content
  ...
```

The engine is a **file server** for skills, not a skill manager. It reads
`skills/<name>/SKILL.md` if present and exposes it as an MCP prompt. No
resolution logic, no defaults, no fallback. Just "if the file exists,
serve it."

### Skills as MCP resources

Skills can also be exposed as MCP resources at `wiki://` URIs:

```
wiki://research/skills/ingest       → skills/ingest/SKILL.md
wiki://research/skills/crystallize  → skills/crystallize/SKILL.md
```

This lets the LLM read skills on demand via `wiki_read` if the client
supports resource access.

### Summary

| What | MCP primitive | Source | Always available |
|------|--------------|--------|------------------|
| Engine docs (`instruct` sections) | Prompt | Embedded in binary | Yes |
| Skill playbooks (`SKILL.md`) | Prompt + Resource | `skills/` on disk | Only if installed |

The engine serves both. It doesn't decide which to use — the LLM or
client picks the right one based on context.

---

## 12. Open Questions

1. **Should `llm-wiki init` create a README in `skills/`?** Something
   like "Install skills from https://github.com/... or write your own.
   See docs for the SKILL.md format." Helps discoverability.

2. **Should `llm-wiki lint` check for the `skills/` folder?** Low
   priority, but could warn if no skills are installed.

3. **MCP resource for skills?** Could expose `wiki://skills/ingest` as
   an MCP resource so the LLM can read skills on demand through the
   engine. But this breaks the "engine doesn't manage skills" principle.
   Probably not.
