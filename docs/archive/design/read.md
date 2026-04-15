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
`wiki://` URI. It is the companion to `wiki search` — search returns
`Vec<PageRef>`, read fetches the content of one.

---

## 1. Input Forms

Both forms resolve to the same page:

```bash
wiki read concepts/mixture-of-experts                        # slug
wiki read wiki://research/concepts/mixture-of-experts        # full URI
wiki read wiki://concepts/mixture-of-experts                 # short URI — default wiki
```

Resolution order:
1. If input starts with `wiki://` — parse wiki name and slug from URI, resolve
   via registry. Short form (`wiki://<slug>`) uses `global.default_wiki`.
2. Otherwise — treat as slug, use default wiki.

---

## 2. Output

Raw Markdown including frontmatter by default:

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

With `--no-frontmatter`, frontmatter is stripped and only the body is returned:

```markdown
## Overview

MoE routes tokens to sparse expert subnetworks...
```

---

## 3. CLI Interface

```
wiki read <slug|uri>
          [--no-frontmatter]   # strip frontmatter from output (default: from config)
          [--wiki <name>]      # override wiki (ignored if URI includes wiki name)
```

### Examples

```bash
wiki read concepts/mixture-of-experts
wiki read wiki://research/concepts/mixture-of-experts
wiki read wiki://concepts/mixture-of-experts --no-frontmatter
wiki read sources/switch-transformer-2021 --wiki research
```

---

## 4. MCP Tool

```rust
#[tool(description = "Read the full Markdown content of a wiki page by slug or wiki:// URI")]
async fn wiki_read(
    &self,
    #[tool(param)] page: String,              // slug or wiki:// URI
    #[tool(param)] no_frontmatter: Option<bool>,
    #[tool(param)] wiki: Option<String>,      // ignored if URI includes wiki name
) -> String { ... }
```

---

## 5. Error Cases

| Condition | Error |
|-----------|-------|
| Slug not found | `error: page not found: concepts/missing` |
| Unknown wiki name in URI | `error: unknown wiki: "unknown"` |
| No default wiki configured | `error: no default wiki set — use --wiki or set global.default_wiki` |

---

## 6. Rust Module Changes

| Module | Change |
|--------|--------|
| `markdown.rs` | Add `read_page(slug, wiki_root, no_frontmatter) -> Result<String>` |
| `registry.rs` | Add `resolve_uri(uri) -> Result<(WikiEntry, slug)>` |
| `cli.rs` | Add `read` subcommand with `<slug|uri>`, `--no-frontmatter`, `--wiki` |
| `server.rs` | Add `wiki_read` MCP tool |

---

## 7. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki read <slug>` | **not implemented** |
| `wiki read <wiki:// URI>` | **not implemented** |
| `--no-frontmatter` flag | **not implemented** |
| `wiki_read` MCP tool | **not implemented** |
