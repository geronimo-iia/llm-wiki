---
title: "Wiki Repository Layout"
summary: "How a wiki repository is structured — wiki.toml, schemas/, three content layers, and the two roots."
read_when:
  - Deciding where to put a new file in the wiki repository
  - Understanding the three-layer DKR structure
  - Understanding what llm-wiki spaces create does
status: ready
last_updated: "2025-07-17"
---

# Wiki Repository Layout

A wiki repository is a git repo with a fixed top-level structure:

```
my-wiki/                    ← git root (repository root)
├── README.md               ← for humans (name, description, usage)
├── wiki.toml               ← wiki config + type registry
├── schemas/                ← JSON Schema per page type
│   ├── base.json
│   ├── concept.json
│   ├── paper.json
│   ├── skill.json
│   ├── doc.json
│   └── section.json
├── inbox/                  ← drop zone (human puts files here)
├── raw/                    ← immutable archive (originals preserved)
└── wiki/                   ← compiled knowledge (authors write here)
```

No hidden directories in the repo. No `schema.md` — `wiki.toml` is the
single source of truth for wiki identity, engine configuration, and the
type registry. See [type-system.md](type-system.md).


## Top-Level Files and Directories

**`wiki.toml`** — wiki identity, engine configuration, and optional
type overrides. The LLM reads it via `wiki_config`.

**`schemas/`** — JSON Schema files (Draft 2020-12) that define
frontmatter per page type. Each schema declares which types it serves
via `x-wiki-types`. The engine discovers types by scanning this
directory — no registration in `wiki.toml` needed for the common case.

**`inbox/`** — human interface. Drop files here for the LLM to process.

**`raw/`** — immutable archive. Originals preserved, never indexed.

**`wiki/`** — compiled knowledge. Authors (human or LLM) write directly
here. Everything inside is a page or asset. The engine indexes it,
searches it, and builds the concept graph from it.


## Folder Structure Inside wiki/

The owner's choice. The engine enforces nothing about categories — only
the `inbox/` → `raw/` → `wiki/` flow matters. Epistemic distinctions
are carried by the `type` field, not by folders. See
[epistemic-model.md](epistemic-model.md).


## Roots

**Repository root** — the git repository directory. Contains
`wiki.toml`, `schemas/`, `inbox/`, `raw/`, and `wiki/`. Created by
`llm-wiki spaces create`.

**Wiki root** — `<repo>/wiki/`. All page slugs are relative to it.
