# Louvain `louvain_phase1`: full ΔQ formula + sigma_tot precomputation

## Decision

Replace the incomplete gain formula in `louvain_phase1` with the full Louvain
ΔQ formula (join gain minus leave cost), and move `sigma_tot` computation out
of the per-node inner loop (precompute once per pass, update incrementally on
move). Both changes land together — the formula fix is a correctness fix; the
sigma_tot change is a performance fix.

## Context

`louvain_phase1` in `src/graph.rs` is the core of community detection used by
`wiki_stats`, `wiki_suggest` (strategy 4: community peers), and the community
map cached in `SpaceContext.community_cache`.

Phase 3 execution revealed that the original algorithm had two distinct
problems, not one:

1. **Correctness bug (discovered during Phase 3):** The original gain formula
   computed only the gain of *joining* candidate community `c`, without
   subtracting the cost of *leaving* `current_c`. This allowed moves that
   decrease modularity, causing oscillation that hit `max_passes` without
   converging to the correct partition. The regression test
   `test_louvain_two_clusters` (two fully-connected clusters of 4 nodes with
   one bridge edge) **failed on the original code** — the plan had assumed it
   would pass.

2. **Performance bug (known before Phase 3):** `sigma_tot` was rebuilt by
   iterating the full `community` map (O(N)) for every node in every pass —
   O(N²) per pass × O(N) passes = O(N³) worst case. Unusable at 20 000 nodes.

## The original formula (incorrect)

```
gain = k_i_in / m  -  sigma_tot[c] * k_i / (2 * m²)
```

This is only the "join" half of the Louvain ΔQ formula. It measures the
modularity gain of adding node `v` to community `c`, but ignores the
modularity loss of removing `v` from `current_c`. A move is accepted whenever
`join_gain > 0`, even if the net modularity change is negative.

## The corrected formula

```
leave_gain = k_i_in_current / m  -  (sigma_tot[current_c] - k_i) * k_i / (2 * m²)
join_gain  = k_i_in / m          -  sigma_tot[c] * k_i / (2 * m²)
net_gain   = join_gain - leave_gain
```

A move is accepted only when `net_gain > 0` — i.e. modularity strictly
increases. `sigma_tot[current_c] - k_i` removes node `v`'s own degree from
the leave-cost calculation (node is leaving, so it should not count itself).

This guarantees modularity strictly increases on every accepted move, prevents
oscillation, and ensures convergence to the correct partition.

## sigma_tot precomputation (performance fix)

In addition to the formula fix, `sigma_tot` is now precomputed once per pass
(O(N)) and updated incrementally on each move, instead of being rebuilt per
node (O(N²) per pass):

- **Precomputed value is exact for all join candidates.** Node `v` is not in
  any candidate community `c ≠ current_c`, so `sigma_tot[c]` does not include
  `v` — exactly what the join formula requires.
- **`sigma_tot[current_c]` includes `v` itself**, but the leave formula
  explicitly subtracts `k_i` (`sigma_tot[current_c] - k_i`), so the
  precomputed value is correct here too.
- **Incremental update after a move** (`sigma_tot[current_c] -= k_i`,
  `sigma_tot[best_c] += k_i`) makes subsequent nodes in the same pass see a
  more accurate `sigma_tot` — moved node no longer contributes to its old
  community's total.

## Alternatives considered

**Fix formula only, keep per-node sigma_tot rebuild.** Correct but still O(N³).
Rejected — the performance fix is straightforward and the two changes are
cleanest together.

**Replace Louvain with a different algorithm** (label propagation, Infomap).
Rejected — Louvain is already implemented and the correctness fix is targeted.
A replacement would require re-validating community quality on real wikis.

**Increase the pass cap** to work around oscillation. Rejected — the cap
addresses infinite loops but does not fix incorrect modularity accounting.
The cap is retained as a hard safety bound after the formula fix.

## Consequences

- `louvain_phase1` now implements the correct full Louvain ΔQ formula.
  Community assignments on graphs where the original formula caused oscillation
  will change — this is the intended behaviour.
- Complexity reduced from O(N³) to O(M × passes) where passes ≤ `n × 10`.
- The v0.2.0 pass cap is retained. The formula fix reduces oscillation in
  practice (moves only accepted when modularity strictly increases), but the
  cap remains as a hard safety bound.
- `test_louvain_two_clusters` added to `src/graph.rs` as a permanent regression
  test. It failed on the original code and passes on the corrected code.
- `sigma_tot` is now a pass-level variable, not a per-node variable. The
  `community` map is no longer iterated inside the node loop.
