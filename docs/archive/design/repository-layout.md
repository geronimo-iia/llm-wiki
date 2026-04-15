---
title: "Repository Layout"
summary: "How llm-wiki organises pages and assets on disk — co-located by default, central assets/ for shared assets only."
read_when:
  - Deciding where to put a new file type in the wiki repository
  - Understanding how slugs map to disk paths
  - Understanding the co-location vs central assets trade-off
status: active
last_updated: "2025-07-15"
---

# Repository Layout

## The rule

A page with no assets is a single `.md` file. A page with assets is a folder
containing `index.md` and its assets beside it. Central `assets/` exists only
for assets explicitly shared across multiple pages.

### Roots

Two distinct roots appear throughout the codebase and docs:

**Wiki root** — the git repository directory for one wiki instance. All page
slugs, asset paths, and index files are relative to it. Configured as `path`
in `~/.wiki/config.toml`. Passed as `wiki_root: &Path` to all engine functions.

**Ingest source root** — the external folder passed to `wiki ingest <path>`.
Used only during ingest to derive page slugs: a file at
`<ingest-source-root>/my-skill/SKILL.md` becomes slug `my-skill/skill` (or
`skills/my-skill` with `--prefix skills`). Has no meaning after ingest
completes — all paths in the wiki are relative to the wiki root.

These two roots are always distinct. The ingest source root is outside the
wiki root. Never confuse them.

### Default categories

`wiki init` creates these directories and they are the only fixed category
prefixes enforced by `validate_slug`. See [epistemic-model.md](epistemic-model.md)
for why each category exists.

| Directory | Purpose | Slug prefix |
|-----------|---------|-------------|
| `concepts/` | Canonical knowledge pages | `concepts/` |
| `sources/` | Per-source summary pages | `sources/` |
| `queries/` | Saved Q&A results | `queries/` |
| `raw/` | Original source files, never modified | not a slug prefix |
| `.wiki/` | Config + search index (gitignored) | not a slug prefix |

Any other directory (e.g. `skills/`, `guides/`) is a **user-defined prefix**
created via `--prefix` during direct ingest. These are not created by
`wiki init` and are not enforced by `validate_slug` for analysis-only ingest.
Direct ingest (`wiki ingest <path> --prefix <name>`) creates them on demand.

```
wiki/                               ← wiki root (git repository)
├── concepts/
│   ├── scaling-laws.md             ← page with no assets: flat file
│   └── mixture-of-experts/         ← page with assets: folder
│       ├── index.md                ← the page
│       ├── moe-routing.png         ← asset co-located
│       └── vllm-config.yaml        ← asset co-located
├── sources/
│   ├── moe-survey-2023.md          ← flat, no assets
│   └── switch-transformer-2021/
│       ├── index.md
│       └── benchmark.py
├── skills/
│   └── semantic-commit/
│       ├── index.md                ← from SKILL.md
│       └── lifecycle.yaml          ← right next to it
├── queries/                        ← always flat (no assets)
│   └── moe-compute-query.md
├── raw/                            ← original source files, never modified
├── assets/                         ← shared assets only
│   └── index.md                    ← auto-generated, committed on every asset ingest
├── LINT.md                         ← committed by wiki lint
└── .wiki/
    ├── config.toml
    ├── index-status.toml           ← committed on every index rebuild
    └── search-index/               ← gitignored, rebuilt on demand
```

---

## Slug resolution

A slug is always a path without extension. The wiki resolves it to a file using
two rules, checked in order:

```
slug: concepts/mixture-of-experts

1. concepts/mixture-of-experts.md        → flat file (no assets)
2. concepts/mixture-of-experts/index.md  → bundle (has assets)
```

The LLM always uses the same slug regardless of which form is on disk. The wiki
handles the resolution transparently.

---

## When to use each form

**Flat file** — page has no assets, or assets are not worth preserving as files
(e.g. a short code snippet is fine as a fenced block in the body).

**Bundle (folder + index.md)** — page has one or more assets that belong to it:
diagrams, configs, scripts, data files. The assets live beside `index.md` with
short relative references (`./moe-routing.png`).

**Central `assets/`** — asset is explicitly referenced by two or more pages.
This is the exception, not the default. If an asset is shared, consider whether
it should instead be promoted to its own concept or source page.

---

## Asset placement in bundles

Assets live directly in the bundle folder alongside `index.md`. No subdirectory
by kind — the folder already provides the namespace:

```
concepts/mixture-of-experts/
├── index.md
├── moe-routing.png         ← not assets/diagrams/moe-routing.png
└── vllm-config.yaml        ← not assets/configs/vllm-config.yaml
```

References from `index.md` are short and stable:

```markdown
![MoE routing](./moe-routing.png)
See [vllm-config.yaml](./vllm-config.yaml)
```

---

## Central `assets/` — shared assets only

When an asset is genuinely shared, it lives under `assets/` with subdirectory
by kind. This is the canonical kind→subdir mapping:

| Kind | Extensions | Subdir |
|------|------------|--------|
| `image` | `.png`, `.jpg`, `.svg`, `.gif` | `assets/diagrams/` |
| `yaml` | `.yaml`, `.yml` | `assets/configs/` |
| `toml` | `.toml` | `assets/configs/` |
| `json` | `.json` | `assets/configs/` |
| `script` | `.py`, `.sh`, `.rs`, `.js` | `assets/scripts/` |
| `data` | `.csv`, `.tsv`, `.jsonl` | `assets/data/` |
| `other` | anything else | `assets/other/` |

Slug = relative path without extension:
`assets/diagrams/moe-routing` → `assets/diagrams/moe-routing.png`

---

## Page discovery

The tantivy indexer, lint pass, graph builder, and MCP resource lister all use
`walkdir`. The page discovery rule:

- A `.md` file named `index.md` → page at slug = parent directory path
- Any other `.md` file → page at slug = path without extension
- Any non-`.md` file inside a bundle folder → asset of that page
- Any non-`.md` file under `assets/` → shared asset

```rust
fn is_page(path: &Path) -> bool {
    path.extension() == Some("md")
}

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

## Assets index

`assets/index.md` tracks shared assets only. It is regenerated and committed on
every ingest that writes at least one shared asset.

Each asset is one row in a Markdown table:

```markdown
| slug | kind | caption | referenced_by |
|------|------|---------|---------------|
| assets/diagrams/moe-routing | image | Token routing diagram | concepts/mixture-of-experts, sources/switch-transformer-2021 |
```

Bundle assets are not in this index — they are discoverable via the bundle folder
and referenced by short relative paths in the page body.

---

## Queries — always flat

Query pages never have co-located assets. They are always flat `.md` files.
If a query result references a diagram, it links to the source page's bundle
asset or a shared asset.

---

## Comparison with previous design

The previous design used central `assets/` for all assets. This was optimised
for the marginal case (shared assets) at the cost of the common case (asset
belongs to one page).

| Concern | Co-location (current) | Central assets/ (previous) |
|---------|----------------------|---------------------------|
| Asset belongs to one page | `./asset.png` — short, stable | `../assets/diagrams/asset.png` — fragile |
| Skill folder ingest | files stay together | files split across two trees |
| `git log` per page | page + assets in one folder | assets scattered |
| Shared asset | explicit `assets/` — rare | default — always |
| LLM relative links | `./asset.png` — obvious | `../assets/diagrams/…` — must know layout |
| `walkdir` complexity | one extra `index.md` guard | simple but wrong default |
