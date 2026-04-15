# Epistemic Model

The wiki organizes knowledge into five categories. Each has a distinct role.
Mixing them collapses distinctions that matter for knowledge quality,
contradiction detection, and provenance tracking.

---

## Origin

The category structure is grounded in Karpathy's LLM Wiki concept (April 2026).
Karpathy's core ideas: process sources at ingest time, detect contradictions,
save Q&A as query-result pages, run lint passes to audit the wiki.

This project extends the model by treating contradictions as permanent
structured pages — not just flags to review, but knowledge nodes richer than
either source alone.

---

## The Five Layers

```
raw/              → what we received        (unprocessed input)
sources/          → what each source claims (provenance)
concepts/         → what we know            (synthesized knowledge)
contradictions/   → where sources disagree  (knowledge structure)
queries/          → what we concluded       (reasoning output)
```

Each layer answers a different question. None can substitute for another.

---

## `raw/` — What We Received

Original source files: PDFs, transcripts, Markdown files, HTML exports.
Never modified. Never indexed. Never included in search or graph traversal.

The wiki is self-contained — you can always re-analyze a source from scratch
without fetching it again. `raw/` is an archive, not a knowledge base.

---

## `sources/` — What Each Source Claims

One page per source document. Records what a specific paper, blog post, or
transcript claims, with what confidence, and where the gaps are.

Exists separately from `concepts/` because contradiction detection requires
knowing *which source made which claim*. If source summaries are merged into
concept pages, that provenance is lost.

```
sources/switch-transformer-2021  → "sparse MoE reduces compute 8x (high confidence)"
sources/moe-survey-2023          → "MoE gains diminish beyond 100B params (medium)"
```

Without `sources/`, you cannot ask "which sources support this claim?" or
"does this source contradict that one?".

---

## `concepts/` — What We Know

One page per concept, continuously enriched across multiple sources. The
canonical answer to "what do we know about X?". Pages accumulate claims,
tags, confidence levels, and contradiction links over time.

The primary retrieval target for `wiki context`. What an LLM reads to answer
a question.

A concept page represents the *current state of knowledge* about a topic.
A source page represents what *one document* said. These are different things.

---

## `contradictions/` — Where Sources Disagree

One page per detected contradiction between sources. Each page carries the
two claims, the dimension of disagreement, the epistemic value, and a status.

**Contradictions are not errors.** They are signal. A contradiction between
two sources reveals the structure of a knowledge domain:

- **Context-dependence** — claim A holds in domain X, claim B in domain Y
- **Time-dependence** — A was true in 2021, superseded by B in 2024
- **Scale-dependence** — A works at small scale, B at large scale
- **Methodology divergence** — different measurement approaches yield different results
- **Genuine open dispute** — the field has not resolved this yet

A contradiction page is richer than either source alone. It is never deleted.
A resolved contradiction still carries the analysis that explains *why* the
sources disagreed — that explanation is the knowledge.

---

## `queries/` — What We Concluded

Saved Q&A results. When an LLM synthesizes an answer from wiki context, that
synthesis is itself knowledge worth preserving.

A query result is not a concept. It is a conclusion drawn from concepts at a
specific point in time, for a specific question. It carries the slugs of the
pages used to produce it — making it auditable and re-derivable.

---

## Why Separation Matters

| If you collapse... | You lose... |
|--------------------|-------------|
| `sources/` into `concepts/` | Provenance — cannot ask "which source claims this?" |
| `contradictions/` | Knowledge structure — domain boundaries become invisible |
| `queries/` into `concepts/` | Auditability — conclusions presented as facts |
| `raw/` into the index | Signal quality — unprocessed content pollutes search |

---

## User-Defined Categories

Beyond the five fixed categories, direct ingest supports user-defined prefixes:
`skills/`, `guides/`, `lessons/`, etc. These are created on demand via
`wiki ingest <path> --prefix <name>`. They are not part of the epistemic model
but are fully indexed and searchable.
