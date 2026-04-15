---
title: "Repository Layout"
summary: "How a wiki repository is structured — four layers, schema.md for conventions, flat file or bundle pages, assets always co-located."
read_when:
  - Deciding where to put a new file type in the wiki repository
  - Understanding how slugs map to disk paths
  - Understanding bundle vs flat page
  - Understanding the four-layer DKR structure
status: active
last_updated: "2025-07-15"
---

# Repository Layout

## The Rule

A page with no assets is a single `.md` file. A page with assets is a folder
containing `index.md` and its assets beside it. Assets always belong to one
page — there is no shared asset folder.

---

## Four-Layer Structure

A wiki repository is a Dynamic Knowledge Repository (DKR). The engine enforces
one flow: inbox → raw → wiki. Everything else is the wiki owner's choice.

```
my-wiki/                    ← git root
├── schema.md               ← wiki conventions (categories, ingest, lint)
├── inbox/                  ← Layer 1: drop zone          (human puts files here)
├── raw/                    ← Layer 2: immutable archive  (moved here after ingest)
├── wiki/                   ← Layer 3: compiled knowledge (LLM writes)
└── .wiki/                  ← Layer 4: engine metadata    (engine writes)
```

**Why this structure:**

- `inbox/` is the explicit human interface — the LLM knows any file here is
  waiting to be processed.
- `raw/` is the immutable archive — provenance is preserved, files are never
  modified after ingest.
- `wiki/` is the compiled knowledge layer. `walkdir` over it needs zero
  exclusions — everything inside is a page or asset.
- `.wiki/` holds engine state (config, search index), separate from all content.
- Git is the activity log. The search engine is the index. No `log.md` or
  `index.md` needed.

**`schema.md`** is the LLM's operating instructions for this wiki instance.
It defines the category structure, ingest depth rules, lint conventions, and
any domain-specific patterns. The engine ships a default template; the owner
customizes it freely. The MCP server injects it at session start — the LLM
always reads it before any operation.

The engine enforces nothing about categories inside `wiki/`. Structure is
entirely defined by `schema.md`.

---

## Roots

Three roots appear throughout the codebase and docs:

**Repository root** — the git repository directory. Contains `schema.md`,
`inbox/`, `raw/`, `wiki/`, and `.wiki/`. This is what `wiki init` creates.

**Wiki root** — `<repo>/wiki/`. All page slugs and asset paths are relative
to it. Configured as `wiki_root` in `.wiki/config.toml`. Passed as
`wiki_root: &Path` to all engine functions.

**Ingest source root** — the external folder passed to `wiki ingest <path>`.
Used only during ingest to derive page slugs. Has no meaning after ingest
completes.

---

## Directory Structure

`wiki init` creates the four-layer skeleton and a default `schema.md`.
The category structure inside `wiki/` is defined by `schema.md` — the
example below uses the default template conventions.

```
my-wiki/                            ← git repository root
├── schema.md                       ← wiki conventions (LLM reads at session start)
├── inbox/                          ← drop zone (human puts files here)
│   └── my-article.md               ← waiting to be ingested
├── raw/                            ← immutable archive (never indexed)
│   └── my-older-article.md         ← already ingested
├── wiki/                           ← wiki root (all slugs relative here)
│   ├── concepts/
│   │   ├── scaling-laws.md         ← flat page (no assets)
│   │   └── mixture-of-experts/     ← bundle (has assets)
│   │       ├── index.md
│   │       ├── moe-routing.png
│   │       └── vllm-config.yaml
│   ├── sources/
│   │   └── switch-transformer-2021.md
│   ├── queries/
│   │   └── moe-routing-comparison.md
│   └── LINT.md                     ← committed by wiki lint
└── .wiki/
    ├── config.toml
    ├── index-status.toml           ← committed on every index rebuild
    └── search-index/               ← gitignored, rebuilt on demand
```

---

## Slug Resolution

A slug is always a path without extension. The wiki resolves it to a file
using two rules, checked in order:

```
slug: concepts/mixture-of-experts

1. concepts/mixture-of-experts.md        → flat file (no assets)
2. concepts/mixture-of-experts/index.md  → bundle (has assets)
```

The LLM always uses the same slug regardless of which form is on disk.

---

## Flat File vs Bundle

**Flat file** — page has no assets, or assets are not worth preserving as
files (a short code snippet is fine as a fenced block in the body).

**Bundle (folder + index.md)** — page has one or more assets. Assets live
beside `index.md` with short relative references:

```
concepts/mixture-of-experts/
├── index.md
├── moe-routing.png
└── vllm-config.yaml
```

```markdown
![MoE routing](./moe-routing.png)
See [vllm-config.yaml](./vllm-config.yaml)
```

---

## Page Discovery

The tantivy indexer, lint pass, graph builder, and MCP resource lister all
use `walkdir` starting at `wiki/`. No exclusions needed — `raw/` and `.wiki/`
are outside the wiki root.

- A `.md` file named `index.md` → page at slug = parent directory path
- Any other `.md` file → page at slug = path without extension
- Any non-`.md` file inside a bundle folder → asset of that page

```rust
fn slug_for(path: &Path, wiki_root: &Path) -> String {
    let rel = path.strip_prefix(wiki_root).unwrap();
    if rel.file_name() == Some("index.md") {
        rel.parent().unwrap().to_string_lossy().into()
    } else {
        rel.with_extension("").to_string_lossy().into()
    }
}
```

---

## Queries — Always Flat

Query pages never have co-located assets. They are always flat `.md` files.
If a query result references a diagram, it links to the source page's bundle
asset via a relative path.
