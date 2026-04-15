---
title: "Read"
summary: "Fetch the full Markdown content of a single wiki page by slug or wiki:// URI."
read_when:
  - Implementing or extending the read command
  - Understanding how slugs and wiki:// URIs resolve to pages
  - Fetching page content in an LLM workflow
status: draft
last_updated: "2025-07-15"
---

# Read

`wiki read` fetches the full Markdown content of a single page by slug or
`wiki://` URI. For bundle pages, it can also list and read co-located assets.
It is the companion to `wiki search` — search returns `Vec<PageRef>`, read
fetches the content of one.

---

## 1. Input Forms

Three forms, all resolved via the registry:

```bash
# Read page content
wiki read wiki://research/concepts/mixture-of-experts        # full URI
wiki read wiki://concepts/mixture-of-experts                 # short URI — default wiki
wiki read concepts/mixture-of-experts                        # slug — default wiki

# List assets of a bundle page
wiki read wiki://research/concepts/mixture-of-experts --list-assets

# Read a specific asset
wiki read wiki://research/concepts/mixture-of-experts/moe-routing.png
```

Resolution order:
1. If input starts with `wiki://` — parse wiki name and slug from URI, resolve
   via registry. Short form (`wiki://<slug>`) uses `global.default_wiki`.
2. Otherwise — treat as slug, use default wiki.
3. If the resolved path is a non-Markdown file inside a bundle — return asset content.

---

## 2. Output

### Page content (default)

Raw Markdown including frontmatter:

```markdown
---
title: "Mixture of Experts"
summary: "Sparse routing of tokens to expert subnetworks."
status: active
tags: [transformers, scaling]
---

## Overview

MoE routes tokens to sparse expert subnetworks...
```

With `--no-frontmatter`, frontmatter is stripped.

### Asset list (`--list-assets`)

Lists co-located assets of a bundle page:

```
wiki://research/concepts/mixture-of-experts/moe-routing.png
wiki://research/concepts/mixture-of-experts/vllm-config.yaml
```

Returns empty list for flat pages (no assets).

### Asset content

When the URI points to an asset file, returns raw bytes (text or binary):

```bash
wiki read wiki://research/concepts/mixture-of-experts/moe-routing.png
# → raw PNG bytes

wiki read wiki://research/skills/semantic-commit/lifecycle.yaml
# → raw YAML text
```

---

## 3. CLI Interface

```
wiki read <slug|uri>
          [--no-frontmatter]   # strip frontmatter from output (default: from config)
          [--list-assets]      # list co-located assets of a bundle page
          [--wiki <name>]      # override wiki (ignored if URI includes wiki name)
```

### Examples

```bash
wiki read wiki://research/concepts/mixture-of-experts
wiki read wiki://concepts/mixture-of-experts --no-frontmatter
wiki read wiki://research/concepts/mixture-of-experts --list-assets
wiki read wiki://research/concepts/mixture-of-experts/moe-routing.png
```

---

## 4. MCP Tool

```rust
#[tool(description = "Read a wiki page or asset by slug or wiki:// URI")]
async fn wiki_read(
    &self,
    #[tool(param)] uri: String,               // slug or wiki:// URI
    #[tool(param)] no_frontmatter: Option<bool>,
    #[tool(param)] list_assets: Option<bool>, // list co-located assets
    #[tool(param)] wiki: Option<String>,      // ignored if URI includes wiki name
) -> String { ... }  // page Markdown, asset list, or raw asset content
```

---

## 5. Error Cases

| Condition | Error |
|-----------|-------|
| Slug not found | `error: page not found: concepts/missing` |
| Asset not found | `error: asset not found: wiki://research/concepts/mixture-of-experts/missing.png` |
| Unknown wiki name in URI | `error: unknown wiki: "unknown"` |
| No default wiki configured | `error: no default wiki set — use --wiki or set global.default_wiki` |
| `--list-assets` on flat page | returns empty list, no error |

---

## 6. Rust Module Changes

| Module | Change |
|--------|--------|
| `markdown.rs` | Add `read_page(slug, wiki_root, no_frontmatter) -> Result<String>` |
| `markdown.rs` | Add `list_assets(slug, wiki_root) -> Result<Vec<String>>` — returns `wiki://` URIs |
| `markdown.rs` | Add `read_asset(slug, filename, wiki_root) -> Result<Vec<u8>>` |
| `registry.rs` | Add `resolve_uri(uri) -> Result<(WikiEntry, slug)>` |
| `cli.rs` | Add `read` subcommand with `<slug|uri>`, `--no-frontmatter`, `--list-assets`, `--wiki` |
| `server.rs` | Add `wiki_read` MCP tool |

---

## 7. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki read <slug\|uri>` — page content | **not implemented** |
| `wiki read <uri> --list-assets` | **not implemented** |
| `wiki read <uri>/<asset>` — asset content | **not implemented** |
| `--no-frontmatter` flag | **not implemented** |
| `wiki_read` MCP tool | **not implemented** |
