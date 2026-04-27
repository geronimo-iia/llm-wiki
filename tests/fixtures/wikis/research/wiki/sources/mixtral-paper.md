---
title: "Mixtral of Experts (2024)"
type: paper
status: active
summary: "Mistral AI paper introducing Mixtral 8x7B, a sparse MoE language model."
tags: [mixture-of-experts, mixtral, mistral-ai, sparse-moe]
confidence: 0.9
last_updated: "2026-01-20"
read_when:
  - studying sparse MoE implementations
  - comparing MoE routing strategies
concepts:
  - concepts/mixture-of-experts
  - concepts/sparse-routing
claims:
  - claim: "Mixtral uses 8 experts with top-2 routing."
    confidence: 0.95
  - claim: "Mixtral 8x7B matches or exceeds Llama 2 70B on most benchmarks."
    confidence: 0.9
---

## Summary

Mixtral of Experts presents Mixtral 8x7B, a sparse mixture-of-experts model
using 8 feed-forward experts per layer with top-2 routing. Despite having 46.7B
total parameters, only 12.9B are active per token.

## Key findings

- Outperforms Llama 2 70B on most standard benchmarks
- 6x faster inference than a dense 70B model at equivalent quality
- Routing is learned, not hand-designed

Also see [[concepts/compute-efficiency]] for compute cost analysis.
