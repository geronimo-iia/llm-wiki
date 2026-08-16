# Mermaid Node IDs from petgraph NodeIndex

## Decision

Use `format!("N{}", idx.index())` as the mermaid node ID for every node in
`render_mermaid`. Remove `mermaid_id` entirely. Labels (`["..."]`) continue
to carry `node.title` unchanged.

## Context

`render_mermaid` derived node IDs by calling `mermaid_id(&node.title)`, which
replaced `/`, `-`, `:` with `_` and handled `://` as a special case. This was
insufficient — page titles can contain spaces, `<`, `>`, `!`, `.`, and other
characters that are invalid in mermaid node IDs, causing parse errors in any
mermaid renderer (issue #128).

Three options were evaluated:

**Option A — sanitize title.** Replace all non-alphanumeric characters with
`_`, collapse consecutive underscores, strip leading/trailing underscores,
prefix `N_` if the result starts with a digit. Produces readable IDs
(`Arc_str_for_Shared_Immutable_Strings`) but introduces collision risk: two
pages with different titles that sanitize identically (e.g. `"Arc<str>"` and
`"Arc str"`) would produce the same ID, silently merging nodes and corrupting
edges in the rendered diagram.

**Option B — sanitize slug.** Slugs contain only `[a-z0-9/\-]` so
sanitization is two replacements. Collision-free by construction since slugs
are unique. Produces readable IDs (`concepts_arc_str`). Requires changing
three call sites from `node.title` to `node.slug`. Does not work for external
nodes — external nodes carry a `wiki://` URI as their slug, which still
requires sanitization.

**Option C — petgraph NodeIndex ordinal.** `idx.index()` is a unique integer
assigned by petgraph at node insertion. `format!("N{}", idx.index())` is
always a valid mermaid ID, requires zero sanitization, and works identically
for local and external nodes. The `mermaid_id` function is deleted entirely.
Downside: mermaid source is opaque (`N0 -->|uses| N7`), but the rendered
diagram is unchanged — labels carry the human-readable title.

## Rationale

Option C was chosen.

**Zero sanitization is the correct abstraction.** The ID is an internal
rendering detail — it exists only to wire nodes to edges in the mermaid
source. It carries no semantic meaning. Deriving it from human-readable text
(title or slug) adds complexity with no benefit to the rendered output.

**Option A has a correctness hole.** Collision between sanitized titles is
not hypothetical — a wiki about Rust will have pages like `Arc<T>` and
`Arc<str>` that sanitize to the same prefix. Silent node merging is worse
than a parse error.

**Option B requires special-casing external nodes.** External nodes use
`wiki://name/slug` as their slug field. That still needs sanitization,
so Option B does not eliminate `mermaid_id` — it only reduces its scope.

**NodeIndex is stable within a render call.** The graph is built once per
`render_mermaid` invocation and not mutated during rendering. Index values
are stable for the lifetime of the call. Across calls the indices may differ
if the graph is rebuilt, but mermaid output is never diffed across calls —
it is rendered and consumed.

**Nobody reads raw mermaid source.** The opaqueness of `N0 -->|uses| N7`
is the only downside. In practice mermaid diagrams are consumed by renderers
(Hugo, GitHub, Obsidian), not read as text.

## Consequences

- `mermaid_id` function deleted from `src/graph.rs`
- `render_mermaid` node loop: `let safe_id = format!("N{}", idx.index())`
- `render_mermaid` edge loop: `format!("N{}", from.index())` / `format!("N{}", to.index())`
- No call site changes needed beyond the two loops above
- External nodes handled identically to local nodes — no special case
- Rendered mermaid diagrams are visually unchanged (labels carry titles)
- Issue #128 resolved permanently with no sanitization logic to maintain
