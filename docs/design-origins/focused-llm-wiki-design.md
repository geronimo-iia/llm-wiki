---
title: "Focused llm-wiki: Wiki Space Management + Search"
summary: "Design document for a focused llm-wiki that exposes only space management, content operations, and search as MCP/ACP/plugin tools — moving all LLM workflow orchestration to skills."
status: draft
last_updated: "2025-07-17"
---

# Focused llm-wiki: Wiki Space Management + Search

llm-wiki becomes a **stateful engine** — it manages files, git, search
indexes, and the concept graph. All intelligence about *how* to use those
primitives (ingest workflows, crystallize, research, bootstrap) moves to
skills that any agent platform can implement differently.

The engine is a dumb pipe. Skills are the brain.

---

## 1. Design Principle

A tool belongs in the engine if and only if it requires **stateful
access** that a skill cannot replicate:

- Filesystem writes into the wiki tree
- Git operations (commit, history)
- Tantivy index queries (search, list, graph traversal)
- Space registry mutations

Everything else — workflow orchestration, LLM prompting, multi-step
procedures — belongs in skills.

---

## 2. MCP/ACP Tool Surface

### Space management (5 tools)

| Tool | Description |
|------|-------------|
| `wiki_init` | Initialize a new wiki repo + register space |
| `wiki_spaces_list` | List all registered wikis |
| `wiki_spaces_remove` | Remove a wiki from the registry |
| `wiki_spaces_set_default` | Set the default wiki |
| `wiki_config` | Get, set, or list configuration values (per-wiki or global) |

### Content operations (5 tools)

| Tool | Description |
|------|-------------|
| `wiki_read` | Read full page content by slug or `wiki://` URI |
| `wiki_write` | Write a file into the wiki tree |
| `wiki_new_page` | Create a page with scaffolded frontmatter |
| `wiki_new_section` | Create a section directory with `index.md` |
| `wiki_commit` | Commit pending changes to git |

### Search & index (6 tools)

| Tool | Description |
|------|-------------|
| `wiki_search` | Full-text BM25 search with optional `--type` filter, returns ranked `PageRef` list |
| `wiki_list` | Paginated page listing with type/status filters |
| `wiki_ingest` | Validate frontmatter + update index + commit |
| `wiki_graph` | Generate concept graph (Mermaid/DOT) from index + links |
| `wiki_index_rebuild` | Rebuild tantivy index from committed files |
| `wiki_index_status` | Check index health (page count, staleness, last build) |

**Total: 16 tools.**

### What was removed from the tool surface

| Former tool | Where it goes | Why |
|-------------|---------------|-----|
| `wiki_lint` | Skill | Read-analyze-fix workflow the LLM orchestrates using `wiki_list`, `wiki_read`, `wiki_search` |
| `wiki_index_check` | Folded into `wiki_index_status` | One status tool is enough |

### Why wiki_config stays

`wiki_config` gives the LLM read/write access to engine configuration
(`wiki.toml` per-wiki, `~/.llm-wiki/config.toml` global).

**Read** — the LLM needs to inspect config to behave correctly: is
`auto_commit` on? What's the default `top_k`? What's the wiki name and
description? A bootstrap skill reads config to orient itself.

**Write** — real use cases exist: an ingest skill may disable
`auto_commit` for a batch operation, a research skill may raise
`search.top_k` for a deep pass, or the user may ask to change a setting
via natural language.

Both are stateful operations on TOML config files with a specific
schema — a skill can't do this without knowing the file locations and
format.

### Why wiki_graph stays

`wiki_graph` requires the engine because it:

1. Reads the tantivy index to collect all pages and their frontmatter
   link fields (`sources`, `concepts`, `superseded_by`, etc.)
2. Reads `x-graph-edges` declarations from type schemas to assign
   relation labels (`fed-by`, `depends-on`, `informs`, `superseded-by`)
3. Parses `[[wiki-links]]` from page bodies (generic `links-to` edges)
4. Builds a petgraph with typed nodes and labeled edges
5. Renders to Mermaid or DOT with subgraph/depth/type/relation filtering

A skill cannot replicate this — it would need to read every page, parse
every link, and build the graph in-context. The engine does it in
milliseconds from the index.

---

## 3. Frontmatter and the Index

The tantivy index is the engine's core data structure. Every frontmatter
field is indexed, making search, list, and graph possible without reading
files from disk.

### What gets indexed

On `wiki_ingest`, the engine parses YAML frontmatter and indexes every
field:

| Field | Index type | Used by |
|-------|-----------|---------|
| `title` | Text (tokenized) | `wiki_search` (BM25 ranking) |
| `summary` | Text (tokenized) | `wiki_search` (BM25 ranking) |
| `read_when` | Text (tokenized, each entry) | `wiki_search` (BM25 ranking) |
| `tldr` | Text (tokenized) | `wiki_search` (BM25 ranking) |
| `tags` | Keyword (exact match per tag) | `wiki_search` (boost), `wiki_list` (future filter) |
| `type` | Keyword (exact match) | `wiki_list --type`, `wiki_graph --type` |
| `status` | Keyword (exact match) | `wiki_list --status` |
| `sources` | Keyword (slug per entry) | `wiki_graph` (edges) |
| `concepts` | Keyword (slug per entry) | `wiki_graph` (edges) |
| `confidence` | Keyword (exact match) | Future filter |
| `owner` | Keyword (exact match) | `wiki_list` (filter by owner) |
| `superseded_by` | Keyword (slug) | `wiki_graph` (supersession edge), `wiki_read` (redirect hint) |
| `last_updated` | Date | Future sort/filter |
| Body text | Text (tokenized) | `wiki_search` (BM25 ranking) |
| Slug | Stored (not searched) | All tools (identifier) |
| URI | Stored (not searched) | All tools (identifier) |

### How frontmatter drives each tool

**`wiki_search`** — BM25 ranks across `title`, `summary`, `read_when`,
`tldr`, `tags`, and body text. The `read_when` field is particularly
valuable: it contains retrieval conditions written as situations, which
match well against natural-language queries.

**`wiki_list`** — Filters on `type` and `status` keyword fields.
Pagination via tantivy offset. Returns `PageSummary` (slug, URI, title,
type, status, tags).

**`wiki_graph`** — Reads `sources`, `concepts`, `superseded_by`, and
other slug-list fields to build directed edges. Each edge gets a
relation label from the type schema's `x-graph-edges` declaration
(`fed-by`, `depends-on`, `informs`, `superseded-by`). Parses
`[[wiki-links]]` from body text as generic `links-to` edges. Renders
the petgraph with optional root/depth/type/relation filters.

**`wiki_read`** — When a page has `superseded_by` set, the output
includes a notice: "This page has been superseded by
`wiki://<slug>`." The LLM or human can then navigate to the
replacement.

**`wiki_ingest`** — Reads the page's `type` field, looks up the
corresponding `[types.<type>]` entry in `wiki.toml` to find the JSON
Schema path, validates frontmatter against it (e.g., `title` + `summary`
for concepts, `name` + `description` for skills). Applies field aliases
via `x-index-aliases` (e.g., `name` → `title` for indexing). Reads
`x-graph-edges` to index edges with relation labels. Sets defaults for
missing optional fields, writes the document to tantivy, commits to git.
See [type-specific-frontmatter](type-specific-frontmatter.md).

### Compatibility with agent-foundation frontmatter

The wiki frontmatter schema is a superset of the agent-foundation doc
frontmatter:

| Agent-foundation field | Wiki field | Indexed? | Notes |
|----------------------|------------|----------|-------|
| `title` | `title` | Yes (text) | Same field, same purpose |
| `summary` | `summary` | Yes (text) | Same field, same purpose |
| `read_when` | `read_when` | Yes (text) | Same field, same purpose — retrieval conditions |
| `status` | `status` | Yes (keyword) | Same field; wiki adds `stub`, `generated` values |
| `last_updated` | `last_updated` | Yes (date) | Same field |
| `owner` | `owner` | Yes (keyword) | Same field — adopted from agent-foundation |
| `superseded_by` | `superseded_by` | Yes (keyword) | Same field — adopted from agent-foundation |

Wiki extends with fields agent-foundation docs don't define:

| Wiki-only field | Purpose | Why it matters for the index |
|----------------|---------|----------------------------|
| `type` | Epistemic classification | Enables type-filtered search and graph |
| `tags` | Search recall | Keyword boost in BM25 |
| `tldr` | Key takeaway | Additional search surface |
| `sources` | Provenance links | Graph edges |
| `concepts` | Concept links | Graph edges |
| `confidence` | Reliability signal | Future filtering |
| `claims` | Structured assertions | Future structured search |

Agent-foundation fields **not** adopted (and why):

| Agent-foundation field | Why excluded |
|----------------------|-------------|
| `requires_skills` | Skill-specific coupling — wiki has no skill system |
| `documents_skill` | Skill-specific coupling |
| `source_study` | Too specific to agent-foundation's task model; wiki uses `sources` |
| `produced_by` | Redundant with git history (commit messages track provenance) |

An agent-foundation doc dropped into a wiki will index correctly on
`title`, `summary`, `read_when`, `status`, `last_updated`, `owner`, and
`superseded_by`. It will lack `type` (defaulted to `page`), `tags`, and
link fields — so it won't appear in type-filtered lists or the concept
graph until enriched.

### The index as the single source of truth for queries

No tool reads `.md` files from disk for queries. All of `wiki_search`,
`wiki_list`, and `wiki_graph` operate on the tantivy index. Only
`wiki_read` goes to disk (to return the full page content). This means:

- Search is fast — tantivy BM25, not file scanning
- List is fast — tantivy collector with keyword filters
- Graph is fast — tantivy collects all link fields, petgraph builds the
  graph in memory
- Ingest is the only write path — it's the gatekeeper for what enters
  the index

If the index is stale (git HEAD moved without ingest), `wiki_index_status`
reports it. `wiki_index_rebuild` reconstructs from committed files.

---

## 4. Claude Code Plugin Structure

```
llm-wiki-plugin/
├── .claude-plugin/
│   └── plugin.json
├── .mcp.json                         ← starts llm-wiki serve
├── skills/                           ← generated from root skills at build time
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
│   └── skill/
│       └── SKILL.md
└── settings.json
```

### plugin.json

```json
{
  "name": "llm-wiki",
  "version": "0.1.0",
  "description": "Git-backed wiki engine — space management, search, knowledge structure.",
  "author": { "name": "geronimo-iia" },
  "license": "MIT OR Apache-2.0"
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

### Skill summary

| Skill | Invocation | Description |
|-------|-----------|-------------|
| `bootstrap` | Auto (session start) | Read wiki.toml (via wiki_config) + hub pages for orientation |
| `ingest` | Manual (`/llm-wiki:ingest`) | Multi-step: read inbox → search existing → synthesize → write → ingest → commit |
| `crystallize` | Manual (`/llm-wiki:crystallize`) | Distil session into durable wiki pages |
| `research` | Auto + manual | Search → read → synthesize answer from wiki knowledge |
| `lint` | Manual (`/llm-wiki:lint`) | Audit structure using list + read, fix orphans and stubs |
| `graph` | Manual (`/llm-wiki:graph`) | Generate and interpret the concept graph |
| `frontmatter` | Auto (background) | Reference content for writing correct frontmatter |
| `skill` | Auto + manual | Find, read, and activate skills stored in the wiki |

All 8 skills come from the `llm-wiki-skills` repo. The plugin is the
single source of truth for workflow instructions.

### What moved from `llm-wiki instruct` to plugin skills

See [§8 llm-wiki-skills Repository](#8-llm-wiki-skills-repository)
for the full mapping.

---

## 5. Non-Claude Agents

The MCP tool surface is agent-agnostic. Non-Claude agents use the same
16 tools and write their own workflow orchestration:

| Agent platform | Workflow layer |
|---------------|---------------|
| Claude Code | Plugin skills (`/llm-wiki:ingest`, etc.) |
| Cursor / Windsurf | MCP tools + custom prompts |
| Custom agents | MCP tools + agent-foundation skills |
| CLI scripts | `llm-wiki` CLI commands directly |

The engine doesn't care who calls the tools or how the workflow is
orchestrated. It validates, indexes, commits, and searches.

---

## 6. What the Engine Binary Contains

After focusing:

| Component | In binary | Notes |
|-----------|-----------|-------|
| CLI commands | Yes | `init`, `new`, `read`, `write`, `ingest`, `search`, `list`, `graph`, `commit`, `index`, `spaces`, `config`, `serve` |
| MCP server (stdio + SSE) | Yes | 16 tools |
| ACP server | Yes | Same 16 tools |
| Tantivy index | Yes | Embedded search engine |
| Git operations (libgit2) | Yes | Commit, diff, history |
| Frontmatter parser | Yes | YAML parsing + field extraction |
| JSON Schema validator | Yes | Type-specific frontmatter validation |
| Petgraph | Yes | Concept graph with typed nodes and labeled edges |
| `llm-wiki instruct` | **Removed** | Replaced by `llm-wiki-skills` plugin repo |
| `llm-wiki lint` (CLI) | **Removed** | Moves to plugin skill |
| `llm-wiki config` (CLI) | Yes | Get/set/list config values |
| Embedded LLM prompts | **None** | Engine has no opinion about LLM behavior |

---

## 7. The Wiki as Skill Registry

The wiki is already a skill registry. No new tools needed.

A `type: skill` page is a wiki page with skill frontmatter (`name`,
`description`, `allowed-tools`, `context`, etc.). On ingest, the engine
validates it against `schemas/skill.json`, aliases `name` → `title` and
`description` → `summary` for indexing, and stores it in tantivy like
any other page.

The existing tools cover the full skill lifecycle:

| Skill operation | Wiki tool | How it works |
|----------------|-----------|-------------|
| **Register** a skill | `wiki_write` + `wiki_ingest` | Write a `type: skill` page, ingest validates + indexes |
| **Discover** skills | `wiki_search --type skill "<query>"` | BM25 search scoped to skill pages |
| **List** all skills | `wiki_list --type skill` | Paginated listing filtered by type |
| **Read** a skill | `wiki_read <slug>` | Returns full content with frontmatter |
| **Update** a skill | `wiki_write` + `wiki_ingest` | Overwrite + re-validate + re-index |
| **Deprecate** a skill | Set `superseded_by` | Points to replacement, `wiki_read` shows notice |
| **Version** a skill | `wiki_commit` | Git history tracks every change |
| **Relate** skills | `concepts`, `[[links]]` | Graph edges between skills and knowledge |

### The missing piece: `--type` filter on wiki_search

`wiki_search` currently does full-text BM25 across all pages.
Adding a `--type` filter (a tantivy keyword filter on the `type` field
combined with the BM25 query) enables scoped search:

```bash
# Find skills relevant to PDF processing
llm-wiki search --type skill "process PDF files"

# Find concepts about routing
llm-wiki search --type concept "routing strategies"

# Find all source pages about transformers
llm-wiki search --type paper,article "transformer architecture"
```

This is a one-line change to the search tool — add a keyword filter
to the tantivy query. No new tool, no new protocol.

### How an agent uses the wiki as a skill registry

An agent that wants to find and use a skill:

```
1. wiki_search(query="process PDF files", type="skill")
   → ranked list of skill pages matching the query

2. wiki_read(<best match slug>)
   → full SKILL.md content with frontmatter

3. Agent parses frontmatter (name, description, allowed-tools, etc.)
   and injects the body into its context
```

This is the same pattern as agent-foundation's progressive disclosure:
discovery (search) → activation (read) → execution (follow
instructions). The wiki provides steps 1 and 2. Step 3 is the agent
runtime's job.

### Why no separate skill protocol

A dedicated skill discovery protocol (like agent-foundation's
`index.json` hub) would duplicate what the wiki already does:

| Hub feature | Wiki equivalent |
|-------------|----------------|
| `index.json` with name + description | `wiki_list --type skill` |
| Search across hubs | `wiki_search --type skill` |
| Fetch skill content | `wiki_read` |
| Version pinning | Git commits |
| Validation | JSON Schema on ingest |

The wiki *is* the hub. The index *is* the registry. Adding a separate
protocol would mean maintaining two systems that do the same thing.

### Cross-wiki skill discovery

`wiki_search --type skill --all "<query>"` searches across all
registered wikis. A team can maintain a shared skill wiki alongside
project-specific knowledge wikis. Skills are discoverable from any
wiki in the space.

### Skills that reference knowledge

Because skills live alongside concepts, sources, and docs in the same
wiki, they can reference knowledge pages:

```yaml
---
name: ingest
description: Process source files into synthesized wiki pages.
type: skill
concepts: [concepts/frontmatter, concepts/epistemic-model]
tags: [ingest, workflow]
---
```

The `concepts` field creates graph edges from the skill to the knowledge
it depends on. `wiki_graph` renders these relationships. An agent can
follow the edges to load supporting knowledge before executing the skill.

---

## 8. llm-wiki-skills Repository

The engine's skills live in a dedicated git repository:
**`llm-wiki-skills`** — a Claude Code plugin that is also usable by
any agent platform.

Skills are decoupled from the engine binary. They evolve faster than
the engine, can be contributed to by the community, and follow the
Claude marketplace and plugin conventions.

### Why a separate repository

| Concern | Embedded in binary | Separate repo |
|---------|-------------------|---------------|
| Release cycle | Coupled to engine releases | Independent — skills ship when ready |
| Contributions | Requires Rust build | PRs to a Markdown repo |
| Distribution | Binary-only | Claude marketplace + git clone |
| Discoverability | Engine-specific | Claude plugin manager + any git client |
| Versioning | Engine semver | Own semver — skills can move faster |
| Testing | Requires engine rebuild | Edit SKILL.md, `/reload-plugins` |

### Repository structure

```
llm-wiki-skills/
├── .claude-plugin/
│   └── plugin.json
├── .mcp.json                         ← starts llm-wiki serve
├── skills/
│   ├── bootstrap/
│   │   └── SKILL.md                  ← session orientation
│   ├── ingest/
│   │   └── SKILL.md                  ← source processing workflow
│   ├── crystallize/
│   │   └── SKILL.md                  ← distil session into wiki pages
│   ├── research/
│   │   └── SKILL.md                  ← search → read → synthesize
│   ├── lint/
│   │   └── SKILL.md                  ← structural audit + fix
│   ├── graph/
│   │   └── SKILL.md                  ← generate and interpret concept graph
│   ├── frontmatter/
│   │   ├── SKILL.md                  ← frontmatter authoring reference
│   │   └── references/
│   │       └── type-taxonomy.md      ← full type taxonomy + per-type templates
│   └── skill/
│       └── SKILL.md                  ← find and activate wiki skills
├── settings.json
├── README.md
├── CHANGELOG.md
└── LICENSE
```

### plugin.json

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

The plugin starts the llm-wiki MCP server. Users must have `llm-wiki`
installed. The plugin provides the skills; the engine provides the
tools.

### Skill inventory

| Skill | Invocation | Description | Tools used |
|-------|-----------|-------------|------------|
| `bootstrap` | Auto (session start) | Read wiki.toml, list types, read hub pages | `wiki_config`, `wiki_list`, `wiki_read` |
| `ingest` | Manual (`/llm-wiki:ingest`) | Process source files into synthesized wiki pages | `wiki_search`, `wiki_read`, `wiki_write`, `wiki_ingest`, `wiki_commit` |
| `crystallize` | Manual (`/llm-wiki:crystallize`) | Distil session into durable wiki pages | `wiki_search`, `wiki_read`, `wiki_write`, `wiki_ingest`, `wiki_commit` |
| `research` | Auto + manual | Search → read → synthesize answer | `wiki_search`, `wiki_read` |
| `lint` | Manual (`/llm-wiki:lint`) | Structural audit + fix | `wiki_list`, `wiki_read`, `wiki_search`, `wiki_write`, `wiki_ingest` |
| `graph` | Manual (`/llm-wiki:graph`) | Generate and interpret concept graph | `wiki_graph`, `wiki_read` |
| `frontmatter` | Auto (background) | Reference content for writing correct frontmatter | `wiki_config` |
| `skill` | Auto + manual | Find, read, and activate wiki skills | `wiki_search`, `wiki_read` |

### Distribution

| Channel | How |
|---------|-----|
| Claude marketplace | Submit to `claude.ai/settings/plugins/submit` |
| Claude Code direct | `claude plugin add https://github.com/geronimo-iia/llm-wiki-skills` |
| Local development | `claude --plugin-dir ./llm-wiki-skills` |
| Other MCP clients | Clone the repo, read `skills/*/SKILL.md` directly |
| Agent-foundation | Clone the repo, skills follow the agentskills.io format |

### Plugin skills vs wiki skills

| Aspect | Plugin skills (llm-wiki-skills) | Wiki skills (`type: skill` pages) |
|--------|--------------------------------|-----------------------------------|
| Source | `llm-wiki-skills` git repo | Written into the wiki tree |
| Scope | Engine-level — how to use the tools | Domain-level — how to do domain tasks |
| Mutable | Via PRs to the repo | Editable in the wiki, versionable |
| Discovery | Claude plugin manager, `/llm-wiki:` namespace | `wiki_search --type skill` / `wiki_list --type skill` |
| Namespacing | `/llm-wiki:<skill>` | Wiki slug |

Plugin skills and wiki skills coexist. An agent bootstraps with plugin
skills (how to use the engine), then discovers wiki skills (how to do
domain work). A wiki skill can extend a plugin skill:

```yaml
---
name: process-paper
description: Process an academic paper into wiki pages.
type: skill
concepts: [concepts/epistemic-model]
tags: [ingest, paper, workflow]
---

# Process Paper

This skill extends the `/llm-wiki:ingest` workflow for academic papers.

1. Follow the ingest workflow
2. Extract claims with confidence levels
3. Create concept pages for new terms
4. Link to existing concept pages
```

### What happened to llm-wiki instruct

`llm-wiki instruct` is replaced by the plugin skills:

| Former instruct workflow | Plugin skill |
|-------------------------|-------------|
| `llm-wiki instruct help` | `bootstrap` |
| `llm-wiki instruct ingest` | `ingest` |
| `llm-wiki instruct research` | `research` |
| `llm-wiki instruct lint` | `lint` |
| `llm-wiki instruct crystallize` | `crystallize` |
| `llm-wiki instruct frontmatter` | `frontmatter` |
| `llm-wiki instruct commit` | Inline in `ingest` and `crystallize` |

`llm-wiki instruct` is removed from the binary. The engine ships no
LLM prompts. The `llm-wiki-skills` repo is the single source of truth
for workflow instructions.

---

## 9. Summary

The focused llm-wiki exposes **16 MCP/ACP tools** in three groups:

- **Space management** (5): init, list, remove, set-default, config
- **Content operations** (5): read, write, new-page, new-section, commit
- **Search & index** (6): search, list, ingest, graph, index-rebuild,
  index-status

Skills live in the **`llm-wiki-skills`** repository — a Claude Code
plugin with 8 skills (bootstrap, ingest, crystallize, research, lint,
graph, frontmatter, skill). Distributed via the Claude marketplace,
git clone, or `--plugin-dir`.

The tantivy index is the query engine. All frontmatter fields are
indexed — `title`, `summary`, `read_when`, `type`, `status`, `tags`,
`sources`, `concepts`, `owner`, `superseded_by`, and body text. This
makes search, filtered listing, and graph traversal possible without
reading files from disk.

All LLM workflow intelligence lives in skills. The Claude Code plugin
ships 6 skills (ingest, crystallize, research, lint, bootstrap,
frontmatter). Other agent platforms write their own.

The engine is stateless with respect to LLM behavior. It has no embedded
prompts, no instruct command, no opinion about how an LLM should use the
tools. It just manages files, git, and search.
