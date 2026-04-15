---
title: "Asset Ingest"
summary: "How llm-wiki handles non-Markdown assets — always co-located with their page in a bundle folder."
read_when:
  - Adding asset support to the ingest pipeline
  - Understanding how direct folder ingest handles non-Markdown files
  - Understanding bundle promotion
status: draft
last_updated: "2025-07-15"
---

# Asset Ingest

Assets are non-Markdown files co-located with their page in a bundle folder.
There is one rule: an asset belongs to the page it lives beside.

If content is referenced by multiple pages, it should be its own concept or
source page — not a shared asset.

---

## 1. One Rule — Co-location

Every asset lives beside its page's `index.md`:

```
wiki://research/skills/semantic-commit
  → skills/semantic-commit/index.md       ← the page
  → skills/semantic-commit/lifecycle.yaml ← asset
  → skills/semantic-commit/install.sh     ← asset

wiki://research/concepts/mixture-of-experts
  → concepts/mixture-of-experts/index.md  ← the page
  → concepts/mixture-of-experts/moe-routing.png ← asset
```

Referenced from the page body via short relative paths:

```markdown
![MoE routing](./moe-routing.png)
See [lifecycle.yaml](./lifecycle.yaml)
```

---

## 2. Direct Folder Ingest

When `wiki ingest <folder> --target wiki://research/skills` encounters a
non-Markdown file, it is co-located with the folder's page automatically:

```
source: ~/agent-skills/semantic-commit/
  SKILL.md          → wiki://research/skills/semantic-commit  (index.md)
  lifecycle.yaml    → wiki://research/skills/semantic-commit/lifecycle.yaml
  install.sh        → wiki://research/skills/semantic-commit/install.sh
```

No configuration needed — proximity implies ownership.

---

## 3. Bundle Promotion

When a flat page gains its first asset, it is promoted automatically from
flat file to bundle:

```
Before:  concepts/mixture-of-experts.md
After:   concepts/mixture-of-experts/index.md
         concepts/mixture-of-experts/moe-routing.png
```

The slug `wiki://research/concepts/mixture-of-experts` continues to resolve
correctly — the resolver checks for `index.md` first.
See [repository-layout.md](repository-layout.md).

---

## 4. Enrichment JSON — Asset Fields

The LLM can declare assets inline in `enrichment.json`. Assets are always
co-located with the page they belong to:

```json
{
  "source": "wiki://research/sources/switch-transformer-2021",
  "assets": [
    {
      "slug": "wiki://research/concepts/mixture-of-experts/moe-routing",
      "filename": "moe-routing.png",
      "kind": "image",
      "content_encoding": "base64",
      "content": "<base64-encoded bytes>",
      "caption": "Token routing in a 4-expert MoE layer"
    }
  ],
  "enrichments": [...],
  "query_results": [...]
}
```

### Asset fields

| Field | Required | Description |
|-------|----------|-------------|
| `slug` | yes | `wiki://` URI of the asset — must be under a valid page slug |
| `filename` | yes | Filename including extension |
| `kind` | no | Inferred from extension if absent |
| `content_encoding` | yes | `utf8` \| `base64` |
| `content` | yes | Asset content |
| `caption` | no | One-line description, indexed for search |

---

## 5. Ingest Pipeline

```
wiki ingest <path> --target wiki://<name>/<section>
  │
  ├─ write pages → {slug}.md or {slug}/index.md
  ├─ write assets → {page-slug}/{filename}  (promote flat→bundle if needed)
  ├─ git commit — +N pages, +M assets
  └─ return IngestReport { pages_written, assets_written, bundles_created }
```

---

## 6. MCP Resources

Assets are exposed as MCP resources under their page's URI:

```
wiki://<name>/concepts/mixture-of-experts/moe-routing.png
wiki://<name>/skills/semantic-commit/lifecycle.yaml
```

---

## 7. Rust Structs

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
    pub slug:             String,          // wiki:// URI
    pub filename:         String,
    pub kind:             Option<AssetKind>,
    pub content_encoding: ContentEncoding,
    pub content:          String,
    pub caption:          Option<String>,
}
```

`Analysis` gains:
```rust
pub assets: Vec<Asset>,
```

---

## 8. Validation Rules

- `slug` must be a valid `wiki://` URI under an existing or to-be-created page slug
- `slug` must not contain `..` or absolute path components
- `content_encoding: base64` → valid base64 required (error)
- `content_encoding: utf8` → valid UTF-8 required (error)
- Slug collision → overwrite silently

---

## 9. Module Impact

| Module | Change |
|--------|--------|
| `analysis.rs` | Add `Asset`, `AssetKind`, `ContentEncoding`; add `assets` to `Analysis` |
| `ingest.rs` | Pass assets through to integrate |
| `integrate.rs` | Add `write_assets` with co-location logic and flat→bundle promotion |
| `markdown.rs` | Add `promote_to_bundle(slug)` — moves `{slug}.md` to `{slug}/index.md` |
| `search.rs` | Update slug resolution to check `index.md` variant |
| `server.rs` | Expose bundle assets as MCP resources |
| `lint.rs` | Report orphan asset references |

---

## 10. Implementation Status

| Feature | Status |
|---------|--------|
| Direct folder ingest — asset co-location | **not implemented** |
| Bundle promotion | **not implemented** |
| `assets` field in enrichment JSON | **not implemented** |
| MCP resource exposure for bundle assets | **not implemented** |
