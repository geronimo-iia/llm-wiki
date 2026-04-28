---
title: "Mixture of Experts"
type: concept
status: active
summary: "Architecture where only a subset of model parameters are active per token."
tags: [mixture-of-experts, scaling, routing]
confidence: 0.9
last_updated: "2026-01-10"
read_when:
  - understanding sparse model architectures
concepts:
  - concepts/sparse-routing
sources:
  - sources/mixtral-paper
claims:
  - claim: "MoE models activate k out of n experts per token."
    confidence: 0.9
---

## Overview

In a Mixture of Experts model, a router network selects which expert sub-networks
process each token. Only k experts are activated per token, reducing compute
relative to a dense model of the same parameter count.

## Key properties

- Parameter count scales without proportional compute increase
- Routing quality determines model quality
- Load balancing across experts is required for stable training

See also [[concepts/sparse-routing]] for routing details.
See [scaling laws](concepts/scaling-laws) for parameter count context.
