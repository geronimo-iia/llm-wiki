# Why Mixture of Experts Is Not Actually Cheap

**Source:** The Gradient blog post, March 2025
**Source type:** article

A common claim in the MoE literature is that sparse models are compute-efficient:
only k of n experts activate per token, so cost scales as O(k/n) of a dense model.

This article argues that claim is misleading in practice.

## The hidden costs

**Memory bandwidth:** All expert weights must reside in memory even though only
k are used. For inference on consumer hardware, memory bandwidth — not FLOP count
— is the bottleneck. A "sparse" 46B parameter model still needs 46B parameters in
memory.

**Routing overhead:** The router itself is a learned linear layer applied to every
token. At scale this is non-trivial.

**Load imbalance:** Without aggressive balancing losses, expert utilization is
uneven. Some experts receive most tokens; others are nearly idle. Effective compute
per parameter drops further.

## Conclusion

MoE models are cheaper in FLOPs per token. They are not cheaper in memory or
wall-clock time unless specifically optimized for sparse inference.

The claim "MoE compute is O(k/n)" is technically correct for FLOPs but
practically misleading for total inference cost.
