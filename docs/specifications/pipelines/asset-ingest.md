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

When `llm-wiki ingest <folder>` encounters a non-Markdown file inside the wiki
tree, it is treated as a co-located asset of the folder's page:

```
wiki tree:  wiki/skills/semantic-commit/
  index.md        → page at slug skills/semantic-commit
  lifecycle.yaml  → asset of skills/semantic-commit
  install.sh      → asset of skills/semantic-commit
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
See [repository-layout.md](../core/repository-layout.md).

---

## 4. Assets via MCP

The LLM can write assets directly into the wiki tree via `wiki_write`:

```
wiki_write("concepts/mixture-of-experts/moe-routing.png", <base64 content>)
```

Assets are always co-located with their page. The LLM writes the asset
file, then runs `wiki_ingest` on the bundle folder to validate and index.

---

## 5. Ingest Pipeline

```
llm-wiki ingest <path>   (path is inside wiki tree)
  │
  ├─ validate .md files → frontmatter checks
  ├─ detect assets → non-.md files in bundle folders
  ├─ index → update tantivy search index
  ├─ git add + commit (if auto_commit = true)
  └─ return IngestReport { pages_validated, assets_found, warnings, commit }
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
```

---

## 8. Validation Rules

- Asset path must not contain `..` or absolute path components
- Asset must be inside a bundle folder (beside an `index.md`)
- Slug collision → overwrite silently

