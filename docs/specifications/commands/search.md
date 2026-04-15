---
title: "Search"
summary: "How wiki search works — single BM25 command with --no-excerpt for reference-only output. Unified PageRef return type. Replaces the separate wiki context command."
read_when:
  - Implementing or extending the search pipeline
  - Understanding the PageRef return type and output modes
  - Integrating wiki search into an LLM workflow
  - Deciding whether wiki context is still needed
status: draft
last_updated: "2025-07-15"
---

# Search

`wiki search` is the single entry point for all retrieval. Full-text BM25
search, always returning `Vec<PageRef>`. Excerpts included by default;
`--no-excerpt` omits them for LLM-oriented workflows.

`wiki context` is superseded by `wiki search --no-excerpt`.

---

## 1. Unified Return Type — `PageRef`

```rust
pub struct PageRef {
    pub slug:    String,
    pub uri:     String,          // wiki://<wiki-name>/<slug>
                                  // wiki://<slug> when targeting the default wiki
    pub title:   String,
    pub score:   f32,             // BM25 relevance score
    pub excerpt: Option<String>,  // Some by default, None with --no-excerpt
}
```

`path` is not included — it is machine-local and always derivable from `uri`
via the spaces config. Use `wiki read <uri>` to fetch the file content.

---

## 2. Output Modes

### Default — with excerpt

```bash
wiki search "mixture of experts"
wiki search "MoE scaling 2021"
```

```
slug:    concepts/mixture-of-experts
uri:     wiki://research/concepts/mixture-of-experts
title:   Mixture of Experts
score:   0.94
excerpt: Sparse routing of tokens to expert subnetworks, trading compute...

slug:    sources/switch-transformer-2021
uri:     wiki://research/sources/switch-transformer-2021
title:   Switch Transformer (2021)
score:   0.81
excerpt: Switch Transformer scales to trillion parameters using sparse MoE...
```

### Reference-only — `--no-excerpt`

```bash
wiki search "how does MoE reduce compute?" --no-excerpt
wiki search "MoE scaling efficiency" --no-excerpt --top-k 3
```

Same results, excerpt omitted. Designed for LLM workflows where the LLM
reviews the reference list and fetches only the pages it needs via `wiki read`.

```
slug:  concepts/mixture-of-experts
uri:   wiki://research/concepts/mixture-of-experts
title: Mixture of Experts
score: 0.94

slug:  queries/moe-routing-decision
uri:   wiki://research/queries/moe-routing-decision
title: MoE Routing — Design Decision
score: 0.87
```

---

## 3. CLI Interface

```
wiki search "<query>"
            [--no-excerpt]        # omit excerpts — refs only
            [--top-k <n>]         # default: from config (built-in: 10)
            [--include-sections]  # include section index pages in results
            [--wiki <name>]       # target specific wiki
            [--all]               # search across all registered wikis
            [--dry-run]           # print query plan, no search
```

---

## 4. MCP Tool

Single tool, single return type:

```rust
#[tool(description = "Search the wiki — returns ranked PageRef list with optional excerpts")]
async fn wiki_search(
    &self,
    #[tool(param)] query: String,
    #[tool(param)] no_excerpt: Option<bool>,
    #[tool(param)] include_sections: Option<bool>,
    #[tool(param)] top_k: Option<u32>,
    #[tool(param)] wiki: Option<String>,
    #[tool(param)] all_wikis: Option<bool>,
) -> Vec<PageRef> { ... }
```

`wiki_context` and `wiki_ask` are removed. `wiki_search` handles both human
and LLM use cases.

