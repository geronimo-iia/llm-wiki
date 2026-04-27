---
title: "Transformer Architecture"
type: concept
status: active
summary: "Attention-based neural network architecture that underlies most modern LLMs."
tags: [transformer, attention, architecture]
confidence: 0.95
last_updated: "2026-01-08"
read_when:
  - understanding the base architecture of language models
concepts:
  - concepts/scaling-laws
claims:
  - claim: "Self-attention computes pairwise token interactions in O(n²) time."
    confidence: 0.95
---

## Overview

The Transformer architecture replaces recurrence with multi-head self-attention,
enabling parallel processing of sequences. It is the foundation of GPT, BERT,
and most production language models.

## Key components

- Multi-head self-attention
- Feed-forward sub-layers
- Residual connections and layer normalization
- Positional encoding
