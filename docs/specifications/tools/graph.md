---
title: "Graph"
summary: "Generate concept graph in Mermaid, DOT, or LLM-readable format."
read_when:
  - Generating concept graphs
  - Visualizing wiki structure
  - Interpreting wiki structure for LLM consumption
status: ready
last_updated: "2026-04-27"
---

# Graph

MCP tool: `wiki_graph`

```
llm-wiki graph
          [--format <fmt>]          # mermaid | dot | llms (default: from config)
          [--root <slug|uri>]       # subgraph from this node
          [--depth <n>]             # hop limit
          [--type <types>]          # comma-separated page types
          [--relation <label>]      # filter edges by relation
          [--output <path>]         # file path (default: stdout)
          [--wiki <name>]
```

| `--root` | `--depth` | Behavior |
|----------|-----------|----------|
| not set | not set | Full graph, all nodes |
| not set | N | Full graph, edges within N hops of any node |
| set | not set | Subgraph from root, default depth from config |
| set | N | Subgraph from root, N hops |

See [graph.md](../engine/graph.md) for the graph engine contract.

> **Note:** This specification is subject to change as the typed graph evolves.

### Output

Mermaid (default):

```
graph LR
  concepts/moe["MoE"]:::concept
  sources/switch["Switch Transformer"]:::paper
  concepts/scaling["Scaling Laws"]:::concept

  sources/switch -->|informs| concepts/moe
  concepts/moe -->|depends-on| concepts/scaling

  classDef concept fill:#cce5ff
  classDef paper fill:#d4edda
```

DOT (`--format dot`):

```dot
digraph wiki {
  "concepts/moe" [label="MoE" type="concept"];
  "sources/switch" [label="Switch Transformer" type="paper"];
  "concepts/scaling" [label="Scaling Laws" type="concept"];
  "sources/switch" -> "concepts/moe" [label="informs"];
  "concepts/moe" -> "concepts/scaling" [label="depends-on"];
}
```

LLM (`--format llms`):

Natural language description of graph structure — directly readable
without a renderer. Surfaces clusters, hubs, relation counts, and
isolated nodes.

```markdown
The wiki graph has 42 nodes and 87 edges across 5 type groups.

**concept** (18 nodes): Agent, Context Window, Mixture of Experts, Scaling Laws, ...
**paper** (14 nodes): Karpathy LLM Wiki, Switch Transformer, ...

Key hubs: Mixture of Experts (12 edges), Scaling Laws (9 edges), Agent (7 edges)

**Edges by relation:**
- `fed-by` (32)
- `depends-on` (28)
- `informs` (18)
- `links-to` (9)

**Isolated nodes (3):** draft-stub-xyz, tangent-note-abc, orphan-page
```

Use `format: "llms"` when the goal is interpretation or analysis.
Use `format: "mermaid"` or `format: "dot"` when a renderable diagram is needed.

A summary line is printed to stderr:

```
graph: 3 nodes, 2 edges
```
