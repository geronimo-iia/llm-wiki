---
title: "Scaling Laws"
type: concept
status: active
summary: "Empirical power-law relationships between model size, data, compute, and loss."
tags: [scaling, training, compute]
confidence: 0.9
last_updated: "2026-01-15"
read_when:
  - planning training runs
  - understanding compute-optimal models
claims:
  - claim: "Loss decreases as a power law with model size, data, and compute."
    confidence: 0.95
---

## Overview

Scaling laws describe how model performance (measured by loss) improves
predictably as model size, dataset size, and compute budget increase.
Chinchilla scaling established that prior large models were undertrained
relative to their parameter count.

## Compute-optimal training

For a given compute budget C, the optimal model size N and token count D
satisfy N ∝ D ∝ √C. This implies most large models should be trained on
more data rather than scaled further in parameters.
