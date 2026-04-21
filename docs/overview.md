---
title: "Overview"
summary: "What llm-wiki is, the problem it solves, the architecture, and how the pieces fit together — engine, type system, skill registry, and plugin skills."
status: ready
last_updated: "2025-07-20"
---

# llm-wiki

A git-backed wiki engine that turns a folder of Markdown files into a
searchable, structured knowledge base. Accessible from the command line,
from any MCP-compatible agent, or from any IDE via ACP.

### Design principles

**No LLM dependency.** The engine manages files, git history, full-text
search, the type system, and the concept graph. It makes no AI calls,
embeds no prompts, and has no opinion about how an LLM should use its
tools. Workflow intelligence lives in skills — external, replaceable,
platform-specific. The engine is a dumb pipe.

**Single binary, zero runtime.** Written in Rust. No garbage collector,
no VM, no Docker, no external services. Tantivy (search), libgit2 (git),
petgraph (graph), and comrak (Markdown) are all compiled in. One binary
does everything.

**Type and content agnostic.** The engine does not know what a
"concept" or a "paper" is. It knows that pages have a `type` field,
that each type has a JSON Schema, and that schemas declare which fields
are indexed and how they relate in the graph. A wiki can store
knowledge pages, agent skills, reference documents, meeting notes, or
anything else — the engine validates and indexes them all uniformly.
The type system is defined by the wiki owner in `wiki.toml` and
`schemas/`, not hardcoded in the binary.

**Document-authority compatible.** The frontmatter schema supports
multiple document conventions in the same wiki. Knowledge pages use
`title`, `summary`, `read_when`, `status`, `owner`, `superseded_by`.
Skill pages use `name`, `description`, `allowed-tools` following the
[agentskills.io](https://agentskills.io) format. The engine doesn't
care which convention a page follows — field aliasing maps different
field names to the same index roles (`name` → `title`, `description`
→ `summary`). Different document authorities coexist, validated by
different JSON Schemas, indexed into the same tantivy fields.

**Plain files, plain git.** The wiki is Markdown files in a git
repository. No database, no proprietary format. Any tool that reads
Markdown can read the wiki. Any tool that reads git can read the
history. The search index is derived — rebuildable from committed files
at any time.

**Skills are separate.** The engine ships no workflow instructions.
The `llm-wiki-skills` repository is a Claude Code plugin with skills
that teach agents how to use the tools. Other agent platforms write
their own skills. The engine and the skills have independent release
cycles, independent contributors, independent distribution.

---

## The Problem

Most AI knowledge tools use RAG: upload documents, ask a question, the
system retrieves relevant text and generates an answer. Each query
starts from scratch. Knowledge does not accumulate.

llm-wiki implements a **Dynamic Knowledge Repository** (DKR): process
sources at ingest time, not query time. The LLM reads each source,
integrates it into the existing wiki — updating concept pages, creating
source summaries, flagging contradictions — and commits the result.
Knowledge compounds with every addition.

|                         | Traditional RAG             | llm-wiki (DKR)                     |
| ----------------------- | --------------------------- | ---------------------------------- |
| When knowledge is built | At query time, per question | At ingest time, once per source    |
| Cross-references        | Discovered ad hoc or missed | Pre-built, continuously maintained |
| Contradiction detection | Never                       | Flagged at ingest time             |
| Knowledge accumulation  | None — resets each query    | Compounds over time                |
| Activity log            | None                        | Git history (semantic commits)     |
| Data ownership          | Provider systems            | Your files, your git repo          |

---

## Architecture

Three independent pieces, three repositories:

```
┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────────┐
│   llm-wiki          │   │   llm-wiki-skills   │   │   llm-wiki-hugo-cms │
│   (engine)          │   │   (plugin)          │   │   (renderer)        │
│                     │   │                     │   │                     │
│   MCP tools         │   │   skills            │   │   Hugo site         │
│   Rust binary       │   │   Claude Code plugin│   │   scaffold          │
│   tantivy + git     │   │   agentskills.io    │   │   GitHub Pages CI   │
└────────┬────────────┘   └────────┬────────────┘   └────────┬────────────┘
         │                         │                          │
         │  MCP/ACP/CLI            │  SKILL.md files          │  reads wiki/
         │                         │                          │
         └─────────┬───────────────┘                          │
                   │                                          │
            ┌──────┴──────┐                                   │
            │  wiki repo  │◄──────────────────────────────────┘
            │  (git)      │
            │             │
            │  wiki.toml  │
            │  schemas/   │
            │  wiki/      │
            │  inbox/     │
            │  raw/       │
            └─────────────┘
```

**llm-wiki** (engine) — a Rust binary that manages wiki repositories.
MCP/ACP tools for space management, content operations, search, and
graph traversal. No embedded LLM prompts. No workflow logic.

**llm-wiki-skills** (plugin) — a Claude Code plugin with skills that
teach agents how to use the engine. Also usable by any agent that reads
SKILL.md files. Distributed via the Claude marketplace, git clone, or
`--plugin-dir`.

**llm-wiki-hugo-cms** (renderer) — a Hugo site scaffold that reads
directly from the wiki tree. The wiki is the CMS, Hugo is the renderer.
Deployed via GitHub Pages.

### Separation of concerns

| Concern                            | Where it lives                              |
| ---------------------------------- | ------------------------------------------- |
| File management, git, search index | Engine (llm-wiki)                           |
| Frontmatter validation             | Engine + JSON Schema files in the wiki repo |
| Concept graph                      | Engine (petgraph from tantivy index)        |
| How to ingest a source             | Skill (llm-wiki-skills)                     |
| How to crystallize a session       | Skill (llm-wiki-skills)                     |
| How to audit wiki structure        | Skill (llm-wiki-skills)                     |
| How to render as a website         | Hugo (llm-wiki-hugo-cms)                    |
| What types exist and their fields  | Wiki repo (`wiki.toml` + `schemas/`)        |

The engine is a dumb pipe. Skills are the brain. The wiki repo is the
state.

---

## The Wiki Repository

A wiki repository is a git repo with a fixed top-level structure:

```
my-wiki/
├── wiki.toml           ← wiki config + type registry
├── schemas/            ← JSON Schema per page type
│   ├── base.json
│   ├── concept.json
│   ├── paper.json
│   ├── skill.json
│   └── ...
├── inbox/              ← drop zone (human puts files here)
├── raw/                ← immutable archive (originals preserved)
└── wiki/               ← compiled knowledge (authors write here)
    ├── concepts/
    ├── sources/
    ├── queries/
    └── skills/
```

**`wiki.toml`** is the single source of truth for wiki identity, engine
configuration, and the type registry. The LLM reads it via
`wiki_config`. No `schema.md` — everything is in `wiki.toml`.

**`schemas/`** contains JSON Schema files (Draft 2020-12) that define
the frontmatter for each page type. The engine validates on ingest.

**`inbox/`** is the human interface — drop files here for the LLM to
process.

**`raw/`** is the immutable archive — originals preserved, never
indexed.

**`wiki/`** is the knowledge layer — authors (human or LLM) write
directly here. Everything inside is a page or asset. The engine indexes
it, searches it, and builds the concept graph from it.

Folder structure inside `wiki/` is the owner's choice. The engine
enforces nothing about categories — only the `inbox/` → `raw/` →
`wiki/` flow matters.

---

## Core Concepts

**Page** — a Markdown file with YAML frontmatter. Either a flat `.md`
file or a bundle folder with `index.md` and co-located assets.

**Slug** — the stable address of a page, derived from its path relative
to `wiki/` without extension. `concepts/mixture-of-experts` resolves to
either `concepts/mixture-of-experts.md` or
`concepts/mixture-of-experts/index.md`.

**`wiki://` URI** — portable reference format.
`wiki://research/concepts/moe` addresses a page in the `research` wiki.
`wiki://concepts/moe` uses the default wiki.

**Write + Ingest** — the two-step pattern. The author writes a file
into the wiki tree, then `llm-wiki ingest` validates frontmatter
against the type's JSON Schema, indexes in tantivy, and commits to git.

**Multi-wiki** — one engine process manages multiple wiki repositories
registered in `~/.llm-wiki/config.toml`. All tools accept
`--wiki <name>`.

---

## The Type System

Every page has a `type` field. The type determines which JSON Schema
validates the frontmatter and how fields are indexed.

### Default types

| Category   | Types                                                                                                   | Epistemic role                              |
| ---------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| Knowledge  | `concept`, `query-result`, `section`                                                                    | What we know, what we concluded, navigation |
| Sources    | `paper`, `article`, `documentation`, `clipping`, `transcript`, `note`, `data`, `book-chapter`, `thread` | What each source claims                     |
| Extensions | `skill`, `doc`                                 ·                                                         | Agent capabilities, reference documents     |

### Type registry

Types are discovered automatically from `schemas/*.json` via
`x-wiki-types`. Most wikis need no `[types.*]` entries in `wiki.toml`.
Overrides are only needed to remap a type to a different schema file:

```toml
[types.paper]
schema = "schemas/my-custom-paper.json"
description = "Custom paper schema with extra fields"
```

### Field aliasing

Different types use different field names for the same role. A concept
has `title` + `summary`. A skill has `name` + `description`. The engine
maps them to the same index fields via `x-index-aliases` in the JSON
Schema:

```json
"x-index-aliases": {
  "name": "title",
  "description": "summary"
}
```

The index is uniform. Search, list, and graph work the same regardless
of page type.

### Typed graph edges

Each type schema declares its outgoing edges via `x-graph-edges`:

```json
"x-graph-edges": {
  "sources":  { "relation": "fed-by",     "target_types": ["paper", "article", ...] },
  "concepts": { "relation": "depends-on", "target_types": ["concept"] }
}
```

The concept graph has typed nodes and labeled edges. `wiki_graph` can
filter by type and relation.

See [type-specific-frontmatter.md](../type-specific-frontmatter.md)
for the full type system specification.

---

## The Epistemic Model

The `type` field carries the distinction between what we know, what
sources claim, and what we concluded:

- **`concept`** — synthesized knowledge from all sources. One concept
  per page. The wiki's compiled understanding.
- **Source types** (`paper`, `article`, `documentation`, etc.) — what
  one specific source claims. Provenance preserved.
- **`query-result`** — a conclusion drawn at a specific point in time.
  Auditable back to sources.

Keeping them separate preserves provenance. A concept page cites its
sources. A source page records what one paper said. A query-result
traces back to both. The graph makes these relationships navigable.

---

## The Tools

The engine exposes MCP/ACP tools in four groups:

| Group              | Tools                                                                                                                  |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| Space management   | `wiki_spaces_create`, `wiki_spaces_list`, `wiki_spaces_remove`, `wiki_spaces_set_default`                              |
| Configuration      | `wiki_config`, `wiki_schema`                                                                                           |
| Content operations | `wiki_content_read`, `wiki_content_write`, `wiki_content_new`, `wiki_content_commit`                                   |
| Search & index     | `wiki_search`, `wiki_list`, `wiki_ingest`, `wiki_graph`, `wiki_index_rebuild`, `wiki_index_status`                     |

Every tool is available via MCP (stdio + HTTP), ACP, and CLI. The same
tool surface, three transports.

A tool belongs in the engine if and only if it requires stateful access
that a skill cannot replicate: filesystem writes, git operations,
tantivy queries, or space registry mutations. Everything else — workflow
orchestration, LLM prompting, multi-step procedures — belongs in skills.

See [specifications/tools/overview.md](specifications/tools/overview.md)
for the complete tool surface.

---

## The Tantivy Index

The search index is the engine's core data structure. Every frontmatter
field is indexed, making search, list, and graph possible without
reading files from disk.

| Field role           | Index type      | Examples                               |
| -------------------- | --------------- | -------------------------------------- |
| Display name         | Text (BM25)     | `title` / `name` (aliased)             |
| Discovery text       | Text (BM25)     | `summary` / `description` (aliased)    |
| Retrieval conditions | Text (BM25)     | `read_when`                            |
| Classification       | Keyword (exact) | `type`, `status`, `confidence`         |
| Search terms         | Keyword (boost) | `tags`                                 |
| Graph edges          | Keyword (slug)  | `sources`, `concepts`, `superseded_by` |
| Ownership            | Keyword (exact) | `owner`                                |
| Body                 | Text (BM25)     | Markdown body                          |

`wiki_search` queries the text fields with BM25 ranking and optional
`--type` keyword filter. `wiki_list` filters on keyword fields.
`wiki_graph` reads edge fields to build the petgraph. `wiki_read` is
the only tool that goes to disk.

Ingest is the only write path — it validates, aliases, indexes, and
commits. If the index is stale, `wiki_index_rebuild` reconstructs from
committed files.

---

## The Wiki as Skill Registry

The wiki is a skill registry. Pages with `type: skill` are searchable,
listable, and readable like any other page. No separate protocol needed.

| Operation         | Tool                                 |
| ----------------- | ------------------------------------ |
| Discover skills   | `wiki_search --type skill "<query>"` |
| List all skills   | `wiki_list --type skill`             |
| Read a skill      | `wiki_read <slug>`                   |
| Register a skill  | `wiki_write` + `wiki_ingest`         |
| Deprecate a skill | Set `superseded_by` in frontmatter   |

An agent finds a skill via search, reads it via `wiki_read`, parses the
frontmatter, and injects the body into its context. The wiki provides
discovery and content. The agent runtime provides execution.

Skills stored in the wiki can reference knowledge pages through
`concepts` and `sources` fields — the graph connects skills to the
knowledge they depend on.

---

## The Plugin Skills (llm-wiki-skills)

The `llm-wiki-skills` repository is a Claude Code plugin that teaches
agents how to use the engine:

| Skill         | Purpose                                                 |
| ------------- | ------------------------------------------------------- |
| `setup`       | Install llm-wiki, create and manage wiki spaces         |
| `bootstrap`   | Session orientation — read config, understand structure |
| `ingest`      | Process source files into synthesized wiki pages        |
| `crystallize` | Distil a session into durable wiki pages                |
| `research`    | Search → read → synthesize from wiki knowledge          |
| `lint`        | Structural audit — orphans, stubs, broken links         |
| `graph`       | Generate and interpret the concept graph                |
| `frontmatter` | Reference for writing correct frontmatter               |
| `skill`       | Find and activate skills stored in the wiki             |
| `write-page`  | Create page of any type                                 |
| `configure-hugo` | Configure wiki for Hugo rendering                    |

Plugin skills are engine-level — they teach how to use the tools.
Wiki skills (`type: skill` pages) are domain-level — they teach how to
do domain work. Both coexist. A wiki skill can extend a plugin skill.

---

## What It Is Not

- **Not an LLM** — makes no AI calls
- **Not a RAG system** — does not retrieve and generate on demand
- **Not a note-taking app** — it is an engine, you bring your own
  interface
- **Not a static site generator** — but llm-wiki-hugo-cms can render
  the wiki as a Hugo site
- **Not a skill runtime** — it stores and discovers skills, agents
  execute them

---

## Project Map

| Repository                                                             | What it is                                         | Language            |
| ---------------------------------------------------------------------- | -------------------------------------------------- | ------------------- |
| [llm-wiki](https://github.com/geronimo-iia/llm-wiki)                   | Wiki engine — 16 MCP tools, tantivy, git, petgraph | Rust                |
| [llm-wiki-skills](https://github.com/geronimo-iia/llm-wiki-skills)     | Claude Code plugin — 8 skills for the engine       | Markdown (SKILL.md) |
| [llm-wiki-hugo-cms](https://github.com/geronimo-iia/llm-wiki-hugo-cms) | Hugo site scaffold — render wiki as a website      | Hugo + HTML         |

---

## Further Reading

| Document                                          | What it covers                                              |
| ------------------------------------------------- | ----------------------------------------------------------- |
| [specifications/](specifications/README.md)       | Detailed specifications per component                       |
| [implementation/](implementation/README.md)       | Implementation notes and module map                         |
| [decisions/](decisions/README.md)                 | Architectural decisions and rationale                        |
| [guides/](guides/README.md)                       | Installation, IDE integration, CI/CD                        |
| [roadmap.md](roadmap.md)                          | Development roadmap                                         |
