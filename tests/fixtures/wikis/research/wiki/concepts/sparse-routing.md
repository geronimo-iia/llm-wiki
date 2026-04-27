---
title: "Sparse Routing"
type: concept
status: active
summary: "Token dispatch mechanism that selects a subset of experts per forward pass."
tags: [mixture-of-experts, routing, efficiency]
confidence: 0.85
last_updated: "2026-01-12"
read_when:
  - understanding how MoE selects experts
concepts:
  - concepts/mixture-of-experts
claims:
  - claim: "Top-k routing assigns each token to exactly k experts."
    confidence: 0.9
  - claim: "Compute cost is O(k/n) relative to a dense model."
    confidence: 0.8
---

## Overview

Sparse routing is the mechanism by which Mixture of Experts models dispatch tokens
to expert sub-networks. A learned router produces logits over all experts; the
top-k experts by logit receive the token.

## Load balancing

Without an auxiliary balancing loss, routers collapse to always selecting the
same few experts. Balancing losses penalize uneven expert utilization.
