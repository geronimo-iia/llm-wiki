# Persistent Knowledge Layer That Refuses to Guess — TDS Article Summary

Source: https://towardsdatascience.com/designing-a-persistent-knowledge-layer-that-refuses-to-guess/
Author: Miodrag Cekikj (TDS, 2026)
Karpathy reference: [LLM Wiki gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) (2025)

## Core argument

Classic RAG retrieves but never accumulates. Semantic caching saves answers, not understanding — the moment a question falls outside the similarity threshold, work starts from zero again. The fix is architectural: add a persistent knowledge layer where reasoning accumulates as typed, linked pages a human can read and audit.

## The three layers

1. **Document / evidence layer** — chunks, embeddings, hybrid search (standard RAG)
2. **Knowledge layer** — wiki: typed pages (concepts, decisions, contradictions, sources), explicit relationships, provenance links back to source documents
3. **Query / synthesis layer** — routes questions to one or both layers, gates on contradictions

## What lives in the knowledge layer

- **Concepts** — named entities with aliases; entity resolution prevents fragmentation into synonyms
- **Decisions** — recorded choices with rationale and source links
- **Contradictions** — explicitly registered when two current documents disagree; never silently resolved
- **Sources** — with provenance (document id, location)
- **Typed relationships** — traversable; enable multi-hop reasoning

## Six failure modes RAG has, the knowledge layer fixes

| # | Failure | RAG behavior | Knowledge layer |
|---|---------|-------------|-----------------|
| 1 | Staleness | Serves superseded chunks silently | Tracks effective/superseded dates; filters before ranking |
| 2 | Genuine contradiction | Picks one document arbitrarily | Raises contradiction, names owner, refuses to answer |
| 3 | Synonym fragmentation | Creates duplicate entities | Entity resolution: id → title → alias matching |
| 4 | Provenance loss | Answer has no document link | Every claim links back to source |
| 5 | Rationale loss | Reasoning lives in email, rule in guideline, connection nowhere | Decision pages capture both rule and rationale in one place |
| 6 | Multi-hop | Top-k similarity ranks, does not traverse | Typed relationships traverse four or more hops |

## Writing knowledge is a different risk class

A wrong chat answer affects one conversation. A wrong canonical concept page corrupts every downstream answer silently, for as long as it stays wrong.

Rules:
- The model never writes to the store directly — it proposes a patch
- The application validates; a human approves consequential changes
- Contradiction resolution is never automatic; if it were, the contradiction object would be pointless

## Query routing

- **Temporal filter before retrieval** — filter by effective date in the query itself; applying it after ranking silently starves top-k on large corpora
- **Three modes**: wiki-only (synthesized knowledge), RAG-only (evidence), hybrid
- **Contradiction gate on exit** — regardless of which mode answered, if the topic is contested the system stops and says so

## Ingestion lifecycle — the detail people miss

When extracting concepts from a new document, the existing wiki must be loaded into the extraction context. Without this, "cash settlement basis", "depreciated value", and "ACV" each become separate pages; the wiki fragments into synonyms.

Entity resolution order: exact id match → title match → alias match. Production systems add embedding similarity + LLM adjudication for the ambiguous middle.

## Azure implementation map

| Concern | Service |
|---------|---------|
| Original documents (immutable) | Blob Storage — versioned, soft-delete 30d, `allowSharedKeyAccess: false` |
| Text extraction | Document Intelligence (layout model) |
| Chunk + vector index | Azure AI Search (hybrid BM25 + vector) |
| Wiki pages | Cosmos DB (NoSQL, serverless) |
| Extraction / synthesis | Azure OpenAI via Foundry |
| API | FastAPI on Container Apps |
| Identity | Managed identity throughout — no connection strings |

Key invariant: the wiki-writing process has no write access to the original document store.

## Entity type schemas (from demo repo)

Demo repo: https://github.com/mcekikj/persistent-knowledge-layer (MIT)

### concept

```yaml
type: concept
concept_id: <slug>
status: provisional | stable
confidence: 0.0–1.0
aliases: [...]
last_validated_at: YYYY-MM-DD
source_ids: [...]
```

Body: summary, evidence quotes, relationship wiki-links.

### decision

```yaml
type: decision
decision_id: <slug>
status: active | superseded
effective_date: YYYY-MM-DD
accountable_owner: <string>
```

Body: rule text, rationale, scope, source links.

### contradiction

```yaml
type: contradiction
contradiction_id: con-NNN
status: unresolved | resolved
severity: high | medium | low
accountable_owner: <string>
raised_on: YYYY-MM-DD
```

Body: `statements[]` — each entry has `source_id`, `locator`, `effective_date`, `statement`.
Contradiction resolution is never automatic.

### source

```yaml
type: source
source_id: <doc-id>
doc_class: underwriting | policy | claims | ...
version: <semver>
effective_date: YYYY-MM-DD
status: current | superseded
superseded_by: <source_id>   # when superseded
superseded_on: YYYY-MM-DD    # when superseded
```

### open_question

Stored as sections in a single `Open Questions.md`; no per-file frontmatter.
Fields inferred from code: `question`, `what_we_know`, `blocked_by` (contradiction id).

### relationship

Not exported to Obsidian as standalone pages — stored in Cosmos only.
Fields: `source` (concept id), `relation` (string), `target` (concept id), `reason`, `confidence`, `evidence_quote`.

### Extraction output schema (LLM → pipeline)

Per document, the model returns:

```json
{
  "summary": "...",
  "concepts": [
    { "name": "...", "summary": "...", "aliases": [], "confidence": 0.9, "evidence_quotes": [] }
  ],
  "relationships": [
    { "source": "...", "relation": "...", "target": "...", "reason": "...", "confidence": 0.8, "evidence_quote": "..." }
  ],
  "comparison_updates": [],
  "open_questions": []
}
```

`comparison_updates` and `open_questions` field shapes not published in the article — defined only in the extraction prompt.

### Cosmos DB layout

All types live in one container, partition key `/workspace_id`, `type` field as discriminator.
Types: `concept`, `decision`, `contradiction`, `source`, `relationship`, `comparison`, `process`, `open_question`.

## Known gaps / what to build next

1. Async ingestion (Event Grid + queues; demo is synchronous)
2. Document Intelligence with page and span preservation for precise citations
3. Strict JSON Schema structured outputs on every extraction and patch
4. Entity resolution with embedding similarity + LLM adjudication (demo does alias matching only)
5. Human approval UI for high-risk patches (lifecycle designed, only low-risk path implemented)
6. Foundry Agent Service tools: propose and apply as separately-permissioned surfaces
7. **Evaluation sets for update accuracy** — open research problem; no good answer yet
8. Freshness and contradiction dashboards (a contradiction register nobody looks at is just a log file)
9. Per-tenant security trimming end to end
10. Model routing by task complexity and risk
