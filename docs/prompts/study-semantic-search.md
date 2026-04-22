# Study: wiki_search hybrid/semantic search

Add embedding-based search alongside BM25 for terminology-independent
retrieval. The single highest-impact feature for LLM agent quality.

## Problem

BM25 is keyword-based. When the LLM asks "what do we know about
scaling efficiency?" it misses pages that use "compute-optimal
training" or "Chinchilla scaling laws" — same concept, different
words.

This is the fundamental limitation of keyword search for knowledge
bases where concepts have multiple names, and the LLM's query
phrasing may not match the author's terminology.

## Approaches

### 1. Tantivy-only (BM25 + query expansion)

No vector store. Improve BM25 with:
- Synonym expansion from the wiki's own tag/concept graph
- Query rewriting: LLM reformulates the query before searching

Pros: no new dependency, no embedding model, no vector index.
Cons: still keyword-based at core, synonym lists need maintenance.

### 2. External vector store (Qdrant, Milvus, etc.)

Separate vector database alongside tantivy. Embed pages at ingest,
query both, merge results.

Pros: best retrieval quality, battle-tested vector search.
Cons: runtime dependency (breaks "single binary, no dependencies"),
operational complexity, embedding model dependency.

### 3. Tantivy + built-in vectors

Tantivy has experimental vector field support (since 0.22). Store
embeddings as tantivy fields, use tantivy's own ANN search.

Pros: single index, no external dependency.
Cons: tantivy's vector support is less mature than dedicated stores,
limited ANN algorithms.

### 4. SQLite + sqlite-vec

Embed vectors in SQLite using the `sqlite-vec` extension. SQLite is
embeddable, no server needed.

Pros: single file, embeddable, no server.
Cons: another dependency (though lighter than Qdrant), sqlite-vec
maturity.

### 5. In-process HNSW (usearch, hora, etc.)

Pure Rust HNSW library for ANN search. Store the index alongside
tantivy.

Pros: no external dependency, single binary, fast.
Cons: less feature-rich than dedicated stores, index management
(persistence, updates) is manual.

## Embedding model

Regardless of vector store choice, we need an embedding model:

### Local (no API dependency)

- `all-MiniLM-L6-v2` via `rust-bert` or `ort` (ONNX Runtime)
- ~80MB model, runs on CPU, 384-dim embeddings
- Pros: offline, no API key, deterministic
- Cons: adds ~80MB to install, CPU inference is slower

### API-based

- OpenAI `text-embedding-3-small`, Cohere, Voyage, etc.
- Pros: high quality, no local model
- Cons: requires API key, network dependency, cost, privacy

### Configurable

Let the user choose via config:

```toml
[search]
embedding_model = "local"           # or "openai", "cohere"
embedding_api_key = ""              # for API-based
```

Recommendation: start with local (`ort` + MiniLM) for zero-config,
add API-based as an option later.

## Hybrid ranking

Combine BM25 and vector scores:

```
final_score = α * bm25_score + (1 - α) * vector_score
```

Where `α` is configurable (`search.hybrid_alpha`, default 0.5).

Reciprocal Rank Fusion (RRF) is an alternative that doesn't require
score normalization:

```
rrf_score = Σ 1 / (k + rank_i)
```

RRF is simpler and more robust. Recommendation: start with RRF.

## Index pipeline changes

At ingest time:
1. Parse frontmatter + body (existing)
2. Generate embedding from `title + summary + body` (new)
3. Store embedding in vector index (new)
4. Index text fields in tantivy (existing)

At search time:
1. BM25 search in tantivy (existing)
2. Vector search in vector index (new)
3. Merge results with RRF (new)

## Interaction with existing features

- Facets: computed from the merged result set
- Cross-wiki search: vector search also runs across all wikis
- `wiki_ingest --dry-run`: skip embedding generation
- Index rebuild: re-embed all pages (slow — cache embeddings?)

## Open questions

- Which vector store approach? (single biggest architectural decision)
- Local vs API embedding model as default?
- Should embedding be opt-in (`search.semantic = true`) or always-on?
- Embedding cache — store embeddings on disk to avoid re-computing
  on rebuild?
- How to handle pages with no body (stubs)? Embed title+summary only?

## Tasks

- [ ] Decide vector store approach
- [ ] Decide embedding model strategy
- [ ] Spec: `docs/specifications/engine/semantic-search.md`
- [ ] Add embedding dependency
- [ ] `src/embeddings.rs` — embedding generation
- [ ] `src/vector_index.rs` — vector store wrapper
- [ ] `src/search.rs` — hybrid search with RRF
- [ ] `src/ingest.rs` — embed at ingest time
- [ ] Config: `search.semantic`, `search.hybrid_alpha`,
  `search.embedding_model`
- [ ] Tests
- [ ] Decision record, changelog, roadmap, skills
