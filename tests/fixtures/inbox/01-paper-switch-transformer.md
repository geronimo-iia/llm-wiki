# Switch Transformer: Scaling to Trillion Parameter Models with Simple and Efficient Sparsity

**Authors:** William Fedus, Barret Zoph, Noam Shazeer
**Published:** 2021 (Google Brain)
**Source type:** paper

## Abstract

The Switch Transformer simplifies the Mixture of Experts architecture by routing
each token to a single expert (top-1 routing) instead of top-2 or more. Despite
the simplicity, it achieves competitive quality with dramatically lower routing
overhead.

## Key contributions

1. Top-1 routing: each token goes to exactly one expert, reducing communication cost
2. Capacity factor: limits how many tokens each expert can process per batch
3. Expert dropout during fine-tuning improves stability
4. Demonstrates scaling to trillion-parameter models

## Findings

- 7x speedup over T5-XXL at equivalent quality
- Sparse models can be pre-trained stably with careful initialization
- Compute cost is sub-linear in parameter count

## Relation to other work

Builds on mixture-of-experts literature. Contrast with Mixtral, which uses top-2
routing and claims better quality-efficiency tradeoff than top-1.

Some sources claim Switch Transformer's top-1 routing produces inferior routing
quality compared to top-2, leading to expert collapse in long training runs.
