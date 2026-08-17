# Louvain `sigma_tot`: precompute per pass, update incrementally on move

## Decision

Move `sigma_tot` computation out of the per-node inner loop in `louvain_phase1`.
Precompute once per pass (O(N)), then update incrementally when a node moves
(subtract `k_i` from the old community, add `k_i` to the new community).
This reduces the algorithm from O(N³) to O(M) per pass, where M is the number
of edges.

## Context

`louvain_phase1` in `src/graph.rs` is the core of community detection used by
`wiki_stats`, `wiki_suggest` (strategy 4: community peers), and the community
map cached in `SpaceContext.community_cache`.

The original implementation rebuilt `sigma_tot` — the sum of degrees of all
nodes in each community — by iterating the full `community` map (O(N)) for
every node in every pass. With N nodes and O(N) passes worst case, this is
O(N³). At 5 000 nodes the latency becomes noticeable; at 20 000 nodes the
algorithm is unusable.

The v0.2.0 pass cap (`n × 10` maximum passes) was introduced to prevent
infinite oscillation when mid-pass moves alter `sigma_tot` for later nodes.
That cap remains in place after this fix.

## Correctness argument

The gain formula for moving node `v` to candidate community `c` is:

```
gain = k_i_in / m  -  sigma_tot[c] * k_i / (2 * m²)
```

where `sigma_tot[c]` is the sum of degrees of all nodes currently in `c`,
and `k_i` is the degree of `v`.

**Precomputed value is exact for all candidates.** `v` is not in any candidate
community `c` (candidates are communities of `v`'s neighbours, excluding
`current_c`). So `sigma_tot[c]` computed before the node loop does not include
`v`, which is exactly what the formula requires.

**`sigma_tot[current_c]` includes `v` itself**, but `current_c` is always
skipped in the gain loop (`if c == current_c { continue; }`), so this value
is never read for the gain calculation.

**Incremental update after a move is more accurate, not less.** When `v` moves
from `current_c` to `best_c`, the update is:

```
sigma_tot[current_c] -= k_i
sigma_tot[best_c]    += k_i
```

Subsequent nodes in the same pass that are also in `current_c` will now see a
`sigma_tot[current_c]` that no longer includes `v`. This is strictly more
accurate than the original per-node rebuild, which always included `v` in
`sigma_tot[current_c]` regardless of whether `v` had already moved.

## Alternatives considered

**Keep per-node rebuild, accept O(N³).** Rejected — unusable at realistic wiki
sizes (20 000 nodes). The pass cap mitigates oscillation but does not change
the complexity class.

**Rebuild `sigma_tot` once per pass, no incremental update.** Correct and
sufficient for O(N) per pass. Rejected in favour of the incremental update
because the incremental version is more accurate for nodes processed later in
the same pass (see above) and costs two map entries per move — negligible.

**Switch to a different community detection algorithm** (e.g. label propagation,
Infomap). Rejected — Louvain is already implemented, tested, and produces
good results on wiki-scale graphs. The fix is a targeted optimisation of the
existing algorithm, not a replacement.

## Consequences

- `louvain_phase1` is O(M) per pass instead of O(N²) per pass. Total complexity
  is O(M × passes), where passes ≤ `n × 10` in the worst case.
- Community assignments may differ slightly from the original on graphs where
  mid-pass moves affect later nodes — this is expected and acceptable. The
  regression test `test_louvain_two_clusters` (two fully-connected clusters of
  4 nodes with one bridge edge) verifies that the algorithm still finds the
  correct partition.
- The v0.2.0 pass cap (`n × 10`) is retained. The incremental update changes
  the oscillation dynamic (moved nodes no longer inflate `sigma_tot` for their
  old community), which may reduce oscillation in practice, but the cap is kept
  as a hard safety bound.
- `test_louvain_two_clusters` added to `src/graph.rs` as a permanent regression
  test. It must pass on both the old and new implementation — verified during
  Phase 3 execution.
