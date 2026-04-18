# Create llm-wiki-skills Repository

## Context

llm-wiki is a git-backed wiki engine that exposes 16 MCP tools for
space management, content operations, search, and graph traversal. The
engine has no embedded LLM prompts — all workflow intelligence lives in
skills.

This session creates the **`llm-wiki-skills`** repository: a Claude
Code plugin that teaches agents how to use the llm-wiki engine. It is
also usable by any agent platform that reads SKILL.md files.

## Design documents to read first

Read these before writing any files:

- `docs/focused-llm-wiki-design.md` — the 16 tools, skill registry,
  plugin structure (§4, §7, §8)
- `docs/type-specific-frontmatter.md` — type system, JSON Schema,
  `wiki.toml` type registry, `x-index-aliases`, `x-graph-edges`
- `docs/roadmap.md` — Phase 1 deliverables for the skills repo
- `docs/specifications/core/frontmatter-authoring.md` — frontmatter
  fields, per-type templates, update rules
- `docs/specifications/commands/cli.md` — full CLI reference
- `docs/specifications/commands/search.md` — search tool details
- `docs/specifications/commands/list.md` — list tool details
- `docs/specifications/pipelines/ingest.md` — ingest pipeline
- `docs/specifications/overview.md` — what llm-wiki is and is not

## Your Task

Create the complete `llm-wiki-skills` repository structure with all
files ready to commit.

## Repository structure to create

```
llm-wiki-skills/
├── .claude-plugin/
│   └── plugin.json
├── .mcp.json
├── skills/
│   ├── bootstrap/
│   │   └── SKILL.md
│   ├── ingest/
│   │   └── SKILL.md
│   ├── crystallize/
│   │   └── SKILL.md
│   ├── research/
│   │   └── SKILL.md
│   ├── lint/
│   │   └── SKILL.md
│   ├── graph/
│   │   └── SKILL.md
│   ├── frontmatter/
│   │   ├── SKILL.md
│   │   └── references/
│   │       └── type-taxonomy.md
│   ├── skill/
│   │   └── SKILL.md
│   ├── write-page/
│   │   └── SKILL.md
│   └── configure-hugo/
│       └── SKILL.md
├── settings.json
├── README.md
├── CHANGELOG.md
└── LICENSE
```

## File specifications

### .claude-plugin/plugin.json

```json
{
  "name": "llm-wiki",
  "version": "0.1.0",
  "description": "Skills for the llm-wiki engine — ingest, search, enrich, audit.",
  "author": { "name": "geronimo-iia" },
  "license": "MIT OR Apache-2.0",
  "repository": "https://github.com/geronimo-iia/llm-wiki-skills",
  "keywords": ["wiki", "knowledge-base", "mcp", "git", "skills"]
}
```

### .mcp.json

```json
{
  "llm-wiki": {
    "command": "llm-wiki",
    "args": ["serve"]
  }
}
```

### settings.json

Empty or minimal. No `agent` override — the plugin adds skills, it
doesn't change Claude's default behavior.

### LICENSE

MIT OR Apache-2.0 dual license (same as llm-wiki engine).

### README.md

Write a README that covers:

- What this plugin is (skills for the llm-wiki engine)
- Prerequisites (`llm-wiki` binary installed)
- Installation (Claude marketplace, `claude plugin add`, `--plugin-dir`)
- Skill inventory table (name, invocation, description)
- How skills relate to the engine's 16 MCP tools
- How to use with non-Claude agents (clone repo, read SKILL.md files)
- Link to the llm-wiki engine repo
- License

### CHANGELOG.md

Initial entry for v0.1.0 with the 10 skills listed.

## Skill specifications

Each SKILL.md must have proper frontmatter following the Claude Code
skill format. Use the agentskills.io compatible subset where possible.

### Common rules for all skills

- Use `description` that matches how users phrase requests — this is
  what Claude uses to decide when to activate the skill
- Include `when_to_use` for additional activation context where needed
- Set `disable-model-invocation: true` for manual-only skills
- List the MCP tools the skill uses in the body, not in `allowed-tools`
  (the tools are MCP server tools, not Claude Code built-in tools)
- Write instructions in third-person imperative tone
- Reference specific MCP tool names: `wiki_search`, `wiki_read`,
  `wiki_write`, `wiki_ingest`, `wiki_commit`, `wiki_list`,
  `wiki_graph`, `wiki_config`, etc.
- Include the orientation step where relevant: search for existing
  pages before writing new ones

### bootstrap

**Invocation**: Auto (session start)
**Purpose**: Session orientation — read wiki config, understand the
wiki's types and structure, read hub pages for context.

Steps:
1. `wiki_config list` — read wiki name, description, types
2. `wiki_list --type section --page-size 50` — get the section
   structure
3. `wiki_read` on key section index pages — understand what knowledge
   exists
4. Summarize the wiki's scope and current state

Frontmatter:
```yaml
---
name: bootstrap
description: >
  Orient to a wiki space. Read configuration, understand types and
  structure, review hub pages. Use at the start of every session or
  when switching wikis.
---
```

### ingest

**Invocation**: Manual (`/llm-wiki:ingest`)
**Purpose**: Process source files into synthesized wiki pages.

Steps:
1. Read the source file (from inbox/ or provided path)
2. `wiki_search` for existing pages on the same topic
3. `wiki_read` existing pages to understand current knowledge
4. Decide: update existing pages or create new ones
5. Write complete Markdown files with frontmatter using `wiki_write`
6. `wiki_ingest` each file — validates, indexes, commits
7. Preserve existing list values (tags, sources, concepts) when
   updating — read before write

Must reference the frontmatter skill for correct field values.

Frontmatter:
```yaml
---
name: ingest
description: >
  Process source files into synthesized wiki pages. Read sources,
  search for existing knowledge, write pages with frontmatter,
  validate and commit. Use when the user drops files in inbox/ or
  says "ingest", "process this", or "add to wiki".
disable-model-invocation: true
argument-hint: "[file-or-folder-path]"
---
```

### crystallize

**Invocation**: Manual (`/llm-wiki:crystallize`)
**Purpose**: Distil the current session into durable wiki pages.

Steps:
1. Review the conversation for decisions, findings, open questions
2. `wiki_search` for existing pages that should be updated
3. `wiki_read` those pages
4. Write updated or new pages — prefer updating hub pages over
   creating orphans
5. `wiki_ingest` + `wiki_commit`

Frontmatter:
```yaml
---
name: crystallize
description: >
  Distil the current session into durable wiki pages. Extract
  decisions, findings, and open questions. Update existing pages
  or create new ones. Use when the user says "crystallize",
  "save this", "write this up", or at the end of a productive session.
disable-model-invocation: true
---
```

### research

**Invocation**: Auto + manual
**Purpose**: Search the wiki, read relevant pages, synthesize an
answer from existing knowledge.

Steps:
1. `wiki_search` with the user's query
2. `wiki_read` the top results
3. Synthesize an answer from the wiki's knowledge
4. Cite sources with `wiki://` URIs
5. Note gaps — what the wiki doesn't cover yet

Frontmatter:
```yaml
---
name: research
description: >
  Search the wiki and synthesize an answer from existing knowledge.
  Use when the user asks a question that the wiki might answer,
  wants to know what the wiki says about a topic, or needs a
  summary of existing knowledge.
---
```

### lint

**Invocation**: Manual (`/llm-wiki:lint`)
**Purpose**: Structural audit — find orphans, missing stubs, empty
sections, broken links.

Steps:
1. `wiki_list` all pages
2. For each page, check:
   - Does it have backlinks? (orphan detection)
   - Do its `sources`/`concepts` slugs resolve? (broken links)
   - If it's a section, does it have children? (empty sections)
3. Report findings
4. Offer to fix: create stubs, add missing links, populate sections
5. `wiki_write` + `wiki_ingest` for fixes

Frontmatter:
```yaml
---
name: lint
description: >
  Audit wiki structure for orphan pages, missing stubs, empty
  sections, and broken links. Offer to fix issues. Use when the
  user says "lint", "audit", "check structure", or "find problems".
disable-model-invocation: true
---
```

### graph

**Invocation**: Manual (`/llm-wiki:graph`)
**Purpose**: Generate and interpret the concept graph.

Steps:
1. `wiki_graph` with optional filters (--type, --root, --depth,
   --relation)
2. Present the Mermaid/DOT output
3. Interpret the graph — identify clusters, isolated nodes, key hubs
4. Suggest improvements — missing links, orphan concepts

Frontmatter:
```yaml
---
name: graph
description: >
  Generate and interpret the wiki's concept graph. Show
  relationships between pages, identify clusters and gaps.
  Use when the user says "graph", "show connections",
  "visualize", or "map the wiki".
disable-model-invocation: true
argument-hint: "[--type concept] [--root slug] [--depth N]"
---
```

### frontmatter

**Invocation**: Auto (background knowledge)
**Purpose**: Reference content for writing correct frontmatter per
page type.

This skill is background knowledge — Claude loads it when writing
wiki pages. It should contain:
- Required fields per type
- Field conventions (title format, summary style, read_when format)
- Per-type templates (concept, paper, article, query-result, skill)
- The accumulation contract (read before write, preserve list values)
- Common mistakes

The `references/type-taxonomy.md` file contains the full type taxonomy
with all 15 default types and their descriptions.

Frontmatter:
```yaml
---
name: frontmatter
description: >
  Reference for writing correct wiki frontmatter. Covers required
  fields, type conventions, per-type templates, and the accumulation
  contract. Use when writing or updating wiki pages.
user-invocable: false
---
```

### skill

**Invocation**: Auto + manual
**Purpose**: Find, read, and activate skills stored in the wiki.

Steps:
1. `wiki_search --type skill "<query>"` — find relevant skills
2. Present the results with name, description, tags
3. `wiki_read` the selected skill — get full instructions
4. Follow the skill's instructions

Frontmatter:
```yaml
---
name: skill
description: >
  Find and activate skills stored in the wiki. Search for skills
  by capability, read their instructions, and follow them. Use
  when the user needs a workflow the wiki might have a skill for,
  or says "find a skill for", "how do I", or "is there a skill".
---
```

### write-page

**Invocation**: Manual (`/llm-wiki:write-page`)
**Purpose**: Create a wiki page of any type with correct frontmatter
and structure.

This is the skill for deliberate page creation — when the user wants
to write a page that isn't driven by source ingestion or session
crystallization. It reads the wiki's type registry, selects the right
template, and guides the agent through type-specific fields.

Steps:
1. Determine the target type from the user's intent (concept, doc,
   section, source, skill, or custom)
2. `wiki_config list` — read registered types and their schemas
3. `wiki_search` for existing pages on the same topic — avoid
   duplicates, find pages to link
4. Determine the slug — ask the user or derive from the title
5. `wiki_content_new <slug> --type <type>` — scaffold the page
   (add `--section` for sections, `--bundle` for pages with assets)
6. Fill in type-specific fields:
   - **section**: title, summary only — no additional fields
   - **concept**: sources, concepts, confidence, claims if known
   - **doc**: sources (informed-by), read_when
   - **source types** (paper, article, etc.): tldr, sources (cites),
     concepts (informs), confidence, claims
   - **skill**: name, description, allowed-tools, disable-model-invocation
7. Write the body content
8. `wiki_content_write <slug>` — save the page
9. `wiki_ingest <slug>` — validate, index, commit
10. If the page should link to/from existing pages, update those pages
    (read before write, preserve existing list values)

Frontmatter:
```yaml
---
name: write-page
description: >
  Create a wiki page of any type. Scaffold frontmatter from the
  type registry, fill in type-specific fields, validate and commit.
  Use when the user says "create a page", "write a doc",
  "add a concept", "new section", or wants to create a page
  outside of ingest or crystallize workflows.
disable-model-invocation: true
argument-hint: "[slug] [--type concept|doc|section|skill|paper|...]"
---
```

### configure-hugo

**Invocation**: Manual (`/llm-wiki:configure-hugo`)
**Purpose**: Configure a wiki for Hugo rendering using the
llm-wiki-hugo-cms scaffold.

The llm-wiki-hugo-cms project provides the base Hugo scaffold (layouts,
shortcodes, CI). This skill configures a *specific wiki* to use it —
reading the wiki's registered types and generating matching Hugo
configuration.

Steps:
1. `wiki_config list` — read wiki name, description, registered types
2. Check if Hugo scaffold exists (hugo.toml, layouts/)
3. If not present, instruct the user to clone/copy llm-wiki-hugo-cms
   into the wiki repo first
4. Generate `hugo.toml` with:
   - `contentDir` pointing to `wiki/`
   - `title` from wiki name
   - `[[cascade]]` rules for each registered type
   - Taxonomy mappings for tags, owner, type
   - `ignoreFiles` for inbox/, raw/, schemas/
5. For each registered type, generate a type-specific layout in
   `layouts/<section>/single.html` if one doesn't exist
6. Generate frontmatter mapping based on type fields (including
   aliases — e.g. skill's `name` → `title`)
7. Validate with `hugo --printUnusedTemplates` if Hugo is installed
8. Report what was generated and what the user should review

Frontmatter:
```yaml
---
name: configure-hugo
description: >
  Configure a wiki for Hugo rendering. Read the wiki's registered
  types, generate Hugo configuration with type-specific cascades
  and layouts, set up frontmatter mapping. Use when the user says
  "set up Hugo", "configure Hugo", "make this wiki publishable",
  or "add Hugo to this wiki". Requires the llm-wiki-hugo-cms
  scaffold to be present in the wiki repo.
disable-model-invocation: true
---
```

## Quality checklist

After creating all files, verify:

- [ ] Every SKILL.md has valid YAML frontmatter
- [ ] Every skill references specific MCP tool names
- [ ] No skill references `llm-wiki instruct` or `schema.md`
- [ ] Skills reference `wiki.toml` and `wiki_config` (not `schema.md`)
- [ ] Manual skills have `disable-model-invocation: true`
- [ ] Background skills have `user-invocable: false`
- [ ] Auto skills have good `description` with trigger phrases
- [ ] `frontmatter` skill content matches
  `docs/specifications/core/frontmatter-authoring.md`
- [ ] `references/type-taxonomy.md` lists all 15 default types
- [ ] `write-page` skill covers all built-in types
- [ ] `write-page` skill references `frontmatter` skill for field details
- [ ] `configure-hugo` skill references llm-wiki-hugo-cms scaffold
- [ ] README includes installation instructions for all channels
- [ ] Plugin structure matches Claude Code plugin conventions
- [ ] Skills follow agentskills.io compatible format where possible
