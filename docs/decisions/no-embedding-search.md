---
title: "No Embedding-Based Semantic Search"
summary: "Why llm-wiki does not add vector embeddings for semantic search — it violates the no-LLM-dependency rule."
status: accepted
last_updated: "2025-07-23"
---

# No Embedding-Based Semantic Search

## Decision

llm-wiki will not add vector embeddings or an embedding model for
semantic search. BM25 (tantivy) remains the only search mechanism
built into the engine.

## Context

The `docs/prompts/study-semantic-search.md` proposed adding vector
embeddings alongside BM25 for better recall on semantically related
queries. This would require:

- An embedding model (e.g. sentence-transformers, OpenAI embeddings)
- A vector index (FAISS, HNSW, or tantivy's vector features)
- Embedding computation at ingest time
- Re-embedding when the model changes

## Why Not

**It violates the core design principle: no LLM dependency.**

The engine manages files, git history, full-text search, the type
system, and the concept graph. It makes no AI calls, embeds no
prompts, and has no opinion about how an LLM should use its tools.

An embedding model is an AI dependency:
- It requires a model binary or API call to produce vectors
- The quality of search depends on the model choice
- Model updates require re-embedding all pages
- It adds a runtime dependency (Python, ONNX, or a service)

This directly contradicts "single binary, zero runtime."

## What Instead

**BM25 covers the common case.** Full-text search with type filters,
facets, and tag boosting handles most queries well. The wiki's
structure (concepts, sources, sections) and the graph (wiki_suggest)
provide semantic navigation that embeddings would approximate.

**QJL-sketch (TurboQuant) is the path to better-than-BM25 without
an embedding model.** The `qjl-sketch` crate compresses and scores
vectors using frozen attention weights from a pretrained model. This
is a one-time weight extraction (not a runtime dependency) and the
scoring is pure math — no model inference at query time.

The pipeline: BM25 pre-filter → QJL attention-based reranking. See
[design-origins/qjl-sketch-pipeline.md](../design-origins/qjl-sketch-pipeline.md).

## Trade-offs

| Concern | Embedding search | BM25 + QJL rerank |
|---------|-----------------|-------------------|
| Runtime dependency | Embedding model required | None (weights extracted once) |
| Query-time inference | Yes (embed the query) | No (matrix multiply) |
| Semantic recall | Good | Good (attention-based) |
| Exact keyword match | Weak | Strong (BM25) |
| Binary size | Larger (model runtime) | Same (compiled in) |
| Model updates | Re-embed everything | Re-extract weights |

## Status

Accepted. The semantic search study prompt remains in `docs/prompts/`
as historical context but is not on the roadmap.
