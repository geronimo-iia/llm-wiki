---
title: "Asset Ingest"
summary: "How llm-wiki handles non-Markdown assets — co-located in the page bundle by default, central assets/ for shared assets only."
read_when:
  - Adding asset support to the ingest pipeline
  - Deciding whether an asset belongs in a bundle or central assets/
  - Extending the analysis.json contract with asset fields
  - Understanding how direct folder ingest handles non-Markdown files
status: draft
last_updated: "2025-07-15"
---

# Asset Ingest

Assets are non-Markdown files worth preserving alongside knowledge. They are
co-located with their page by default. Central `assets/` is for shared assets
only. See [repository-layout.md](repository-layout.md) for the full layout.

---

## 1. Two Asset Placements

**Co-located (default)** — asset belongs to one page. Lives in the page's bundle
folder beside `index.md`. Referenced via short relative path `./asset.png`.

**Shared (explicit)** — asset is referenced by two or more pages. Lives under
`assets/{subdir}/`. Referenced via path from wiki root.

The ingest pipeline decides placement based on `referenced_by`:
- `referenced_by` empty or one entry → co-located in the owning page's bundle
- `referenced_by` has two or more entries → shared, goes to `assets/{subdir}/`

For direct folder ingest, all non-Markdown files in a folder are co-located with
the folder's `index.md` — no `referenced_by` needed.

---

## 2. Two Asset Sources

**Source A — analysis JSON**: the external LLM declares assets with inline
content in `analysis.json`. Placement determined by `referenced_by` count.

**Source B — direct folder ingest**: non-Markdown files encountered during
`wiki ingest <folder>` are co-located with the folder's page. No LLM step.
See [ingest.md § 4](ingest.md).

Both sources produce a committed file and an entry in the relevant index.

---

## 3. analysis.json Extension

A new top-level field `assets` is added to the `Analysis` struct:

```json
{
  "source": "...",
  "assets": [
    {
      "slug": "concepts/mixture-of-experts/moe-routing",
      "filename": "moe-routing.png",
      "kind": "image",
      "content_encoding": "base64",
      "content": "<base64-encoded bytes>",
      "caption": "Token routing in a 4-expert MoE layer",
      "referenced_by": ["concepts/mixture-of-experts"]
    },
    {
      "slug": "assets/diagrams/transformer-overview",
      "filename": "transformer-overview.png",
      "kind": "image",
      "content_encoding": "base64",
      "content": "<base64-encoded bytes>",
      "caption": "Transformer architecture overview",
      "referenced_by": [
        "concepts/mixture-of-experts",
        "concepts/scaling-laws",
        "sources/attention-is-all-you-need"
      ]
    }
  ]
}
```

The first asset has one `referenced_by` entry → co-located in
`concepts/mixture-of-experts/moe-routing.png`.

The second has three → shared, written to `assets/diagrams/transformer-overview.png`.

### Asset fields

| Field | Required | Description |
|-------|----------|-------------|
| `slug` | yes | Target path without extension. For co-located: `{page-slug}/{stem}`. For shared: `assets/{subdir}/{stem}` |
| `filename` | yes | Filename including extension |
| `kind` | no | Inferred from extension if absent |
| `content_encoding` | yes | `utf8` \| `base64` |
| `content` | yes | Asset content |
| `caption` | no | One-line description |
| `referenced_by` | yes | Page slugs that reference this asset — determines placement |

---

## 4. Direct Folder Ingest — Asset Handling

When `wiki ingest <folder>` encounters a non-Markdown file, it is co-located
with the folder's page:

```
agent-skills/semantic-commit/
├── SKILL.md          → skills/semantic-commit/index.md
├── lifecycle.yaml    → skills/semantic-commit/lifecycle.yaml  (co-located)
└── install.sh        → skills/semantic-commit/install.sh      (co-located)
```

The page bundle is created automatically. No `referenced_by` needed — proximity
implies ownership.

---

## 5. Bundle Creation

When the first asset is co-located with a page, the page is promoted from flat
file to bundle:

```
Before:  concepts/mixture-of-experts.md
After:   concepts/mixture-of-experts/index.md
         concepts/mixture-of-experts/moe-routing.png
```

The wiki handles this promotion automatically during ingest. The slug
`concepts/mixture-of-experts` continues to resolve correctly — the resolver
checks for `index.md` first (see [repository-layout.md](repository-layout.md)).

---

## 6. Ingest Pipeline

```
wiki ingest <path>
  │
  ├─ write pages → {slug}.md or {slug}/index.md
  ├─ write contradictions/*.md
  ├─ write assets:
  │    co-located → {page-slug}/{filename}  (promote flat→bundle if needed)
  │    shared     → assets/{subdir}/{filename}
  │    update assets/index.md (shared assets only)
  ├─ git commit — +N pages, +M assets
  └─ return IngestReport { assets_written, bundles_created, ... }
```

---

## 7. Rust Struct Additions

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Image, Yaml, Toml, Json, Script, Data, Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentEncoding { Utf8, Base64 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub slug:             String,
    pub filename:         String,
    pub kind:             Option<AssetKind>,
    pub content_encoding: ContentEncoding,
    pub content:          String,
    pub caption:          Option<String>,
    pub referenced_by:    Vec<String>,
}
```

`Analysis` gains:
```rust
pub assets: Vec<Asset>,
```

`IngestReport` gains:
```rust
pub assets_written: usize,
pub bundles_created: usize,   // flat pages promoted to bundles
```

---

## 8. Validation Rules

- `slug` must not contain `..` or absolute components
- Co-located slug must start with a valid page slug prefix
- Shared slug must start with `assets/` and match the kind→subdir table in
  [repository-layout.md](repository-layout.md)
- `content_encoding: base64` → valid base64 required (error)
- `content_encoding: utf8` → valid UTF-8 required (error)
- Slug collision → overwrite silently
- `referenced_by` slugs not validated at ingest time

---

## 9. Page References to Assets

Co-located assets use short relative paths:

```markdown
![MoE routing](./moe-routing.png)
See [lifecycle.yaml](./lifecycle.yaml)
```

Shared assets use paths from the wiki root:

```markdown
![Transformer overview](../../assets/diagrams/transformer-overview.png)
```

The wiki does not inject references automatically. The LLM writes them in `body`;
direct ingest page authors write them manually.

---

## 10. MCP Resources

```
wiki://<name>/concepts/mixture-of-experts/moe-routing.png  → co-located asset
wiki://<name>/assets/diagrams/transformer-overview.png     → shared asset
wiki://<name>/assets/index                                 → shared assets index
```

---

## 11. Search Integration

Co-located assets are discoverable via their page — searching for the page
surfaces the bundle, and the MCP resource list for the page includes its assets.

Shared assets are indexed via `assets/index.md`. Asset captions are indexed
fields. `wiki context` can surface shared assets when a query matches a caption.

---

## 12. Module Impact

| Module | Change |
|--------|--------|
| `analysis.rs` | Add `Asset`, `AssetKind`, `ContentEncoding`; add `assets` to `Analysis` |
| `ingest.rs` | Pass assets through to integrate for both modes |
| `integrate.rs` | Add `write_assets` with co-location logic and flat→bundle promotion; add `bundles_created` to report |
| `markdown.rs` | Add `promote_to_bundle(slug)` — moves `{slug}.md` to `{slug}/index.md` |
| `search.rs` | Update slug resolution to check `index.md` variant |
| `context.rs` | Update slug resolution |
| `server.rs` | Expose bundle assets as MCP resources; expose `assets/**` for shared |
| `lint.rs` | Report orphan asset references |

---

## 13. Open Questions

1. **Size limits** — base64 images in `analysis.json` can be large. Enforce a
   per-asset byte limit, or require large assets to be provided as file paths?

2. **Asset deduplication** — same slug, different content: silent overwrite or
   hash-based warning?

3. **Binary assets in MCP** — returning base64 images as MCP resource content
   is non-standard. Serve via HTTP endpoint (`wiki serve --sse`) instead?
