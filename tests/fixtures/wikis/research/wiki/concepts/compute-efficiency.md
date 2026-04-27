---
title: "Compute Efficiency"
type: concept
status: draft
summary: "Techniques to reduce FLOPs per token while preserving model quality."
tags: [efficiency, compute, mixture-of-experts]
confidence: 0.5
last_updated: "2026-02-01"
read_when:
  - evaluating inference cost
concepts:
  - concepts/mixture-of-experts
  - concepts/scaling-laws
claims:
  - claim: "MoE compute cost is O(n) not O(k/n) relative to a dense model."
    confidence: 0.4
---

## Overview

Compute efficiency research focuses on reducing the FLOPs required per useful
output token. Sparse architectures like MoE are one approach; quantization
and distillation are others.

## Open questions

- Is MoE compute actually O(k/n) or O(n)? Sources disagree.
  See [[concepts/sparse-routing]] for the conflicting claim.
