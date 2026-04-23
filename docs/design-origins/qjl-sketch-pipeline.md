---
title: "QJL-Sketch Pipeline"
summary: "Search query → BM25 pre-filter → QJL attention scoring → ranked pages. No LLM needed for retrieval."
read_when:
  - Understanding how qjl-sketch integrates with llm-wiki
  - Understanding the alternative to RAG
status: proposal
last_updated: "2025-07-23"
---

# QJL-Sketch Pipeline

Integrates the `qjl-sketch` crate with llm-wiki to replace vector DB
retrieval with attention-based scoring over compressed KV stores.

## Overview

The search query is the attention goal. BM25 (tantivy) does the coarse
pre-filter. QJL scores the candidates using a frozen attention
head — no LLM, no embedding model, no vector DB.

```
User query
    │
    ▼
┌──────────────────────────────────────────────────┐
│  1. SEARCH — wiki_search (BM25, tantivy)         │
│     query → ranked page slugs + excerpts         │
└──────────────┬───────────────────────────────────┘
               │ top-k slugs
               ▼
┌──────────────────────────────────────────────────┐
│  2. COMPRESS — build QJL KV store         │
│                                                  │
│     Each page is projected into K and V vectors  │
│     using a frozen attention head (no full LLM): │
│     - K = W_k · page_tokens                      │
│     - V = W_v · page_tokens                      │
│                                                  │
│     Then qjl-sketch compresses:                  │
│     - QJL sign hashing (keys)                    │
│     - Min-max quantization (values)              │
│                                                  │
│     One-time cost per page version.              │
│     Stored in keys.bin / values.bin alongside    │
│     the tantivy index.                           │
│     Re-compress only when the page changes.      │
└──────────────┬───────────────────────────────────┘
               │ compressed K, V per page
               ▼
┌──────────────────────────────────────────────────┐
│  3. ATTEND — attention kernel (no LLM)           │
│                                                  │
│     Q = W_q · query_tokens                       │
│                                                  │
│     For each candidate page's compressed K:      │
│       scores = QJLSketch::score(Q, compressed_K) │
│                                                  │
│     This is just signed dot products + norms.    │
│     No generation, no decoding, no LLM runtime.  │
│                                                  │
│     The attention scores ARE the relevance       │
│     ranking — which pages matter for this query. │
└──────────────┬───────────────────────────────────┘
               │ per-page scores
               ▼
┌──────────────────────────────────────────────────┐
│  4. RANK + GROUND — results with provenance      │
│                                                  │
│     Aggregate scores per page → ranked list.     │
│     Each result carries wiki:// URI, frontmatter │
│     (sources, concepts, status).                 │
│                                                  │
│     No hallucinated citations — the pages were   │
│     literally scored against the query.          │
└──────────────────────────────────────────────────┘
```

## Why this doesn't need an LLM

Attention is Q·Kᵀ → softmax → weighted V. Three matrix operations.
You need:

- Projection weights W_q, W_k, W_v (frozen, from any pretrained model)
- The QJL score kernel (from the `qjl-sketch` crate)
- A softmax

No autoregressive decoding. No token generation. No sampling. No
full model in memory. Just the attention head weights and the
compressed KV store.

If you later want to generate an answer (not just rank), feed the
top-ranked pages into an LLM as a second step. Retrieval itself is
LLM-free.

## Integration with llm-wiki

### Ingest path

`wiki_ingest` already validates frontmatter, updates the tantivy index,
and commits to git. The qjl-sketch step hooks in after indexing:

```
wiki_ingest
    │
    ├─ validate frontmatter
    ├─ update tantivy index
    ├─ commit to git
    └─ compress page → append to KeyStore + ValueStore
```

The compressed KV store lives alongside the tantivy index:

```
~/.llm-wiki/indexes/<wiki-name>/
├── tantivy/            ← existing search index
├── keys.bin            ← qjl-sketch compressed keys
├── keys.idx
├── values.bin          ← qjl-sketch compressed values
└── values.idx
```

### Query path

`wiki_search` returns BM25 results. A new `wiki_rerank` tool (or a
`--rerank` flag on `wiki_search`) scores the BM25 candidates with
qjl-sketch:

```
wiki_search "mixture of experts" --top-k 50
    │
    ▼ 50 BM25 candidates
wiki_rerank (QJL attention scores)
    │
    ▼ re-ranked top 10
```

### Staleness

The KV store uses content hashing (blake3). On ingest, if the page
content hash changed, re-compress. Same pattern as tantivy index
staleness.

## The BM25 pre-filter

| Wiki size | Strategy |
|-----------|----------|
| < budget | Load everything. No pre-filter needed. |
| 1–5x budget | BM25 top-k. Generous k, let attention sort it out. |
| > 5x budget | BM25 top-k + graph expansion (wiki_suggest). |

The pre-filter is coarse and fast. Attention does the fine-grained
relevance ranking.

## Graph expansion

After BM25 returns top-k pages, expand via the concept graph:

```
BM25 top-k slugs
    │
    ▼
wiki_suggest (graph neighborhood, 1-2 hops)
    │
    ▼
Add linked pages that fit within remaining budget
```

## What changes vs. traditional RAG

| Concern | RAG | This pipeline |
|---------|-----|---------------|
| Retrieval model | Separate embedding model | Frozen attention head weights |
| Relevance ranking | Cosine similarity (approximate) | Attention scores (near-lossless) |
| Context fragments | Chunks, often missing context | Full synthesized pages |
| Provenance | Chunk ID → hope it maps back | wiki:// URI, frontmatter |
| Knowledge freshness | Re-embed on update | Re-compress on page change |
| Infrastructure | Vector DB + embedding service | tantivy + qjl-sketch (compiled in) |
| LLM required | Yes (for generation) | No. Optional for answer generation. |

## Dependencies

The `qjl-sketch` crate provides:
- `QJLSketch` — projection, quantization, scoring
- `KeyStore` / `ValueStore` — persistence with mmap
- `CompressedKeys` / `CompressedValues` — data structures

llm-wiki adds:
- BM25 pre-filter (tantivy, already exists)
- Graph expansion (wiki_suggest, already exists)
- Ingest hook (trigger compress on page change)
- W_q / W_k / W_v weight loading (from GGUF or safetensors)
- Tokenization (page content → token IDs)

## Open questions

- Which pretrained model to extract W_q, W_k, W_v from?
- How to tokenize page content without a full model runtime?
- Should `wiki_rerank` be a separate tool or a flag on `wiki_search`?
- How to handle multi-head attention (aggregate scores across heads)?
