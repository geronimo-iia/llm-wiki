# Task — wiki instruct: doc authoring + enrichment contract

Update `src/instructions.md` and the `wiki instruct` command to cover two new
topics: how to write a wiki document (doc authoring conventions), and how to
produce `enrichment.json` (the revised analysis contract from `design-evolution.md`).

Depends on: `design-evolution.md` decisions being stable.

---

## Background

`wiki instruct` currently prints one monolithic guide covering ingest, research,
contradiction, and lint workflows. It references the old `analysis.json` contract
(LLM writes page bodies via `suggested_pages`).

Two things are missing:

1. **Doc authoring instructions** — when an LLM creates or edits a wiki page
   directly (e.g. a query result, a stub enrichment), it needs to know the
   frontmatter schema, `read_when` discipline, `summary` vs `title` distinction,
   status values, and bundle vs flat layout rules.

2. **Enrichment contract** — the new `enrichment.json` schema (`enrichments[]`,
   `query_results[]`, `contradictions[]`) replaces the old `suggested_pages[]`
   contract. The LLM needs precise instructions on what to extract and how to
   populate each field.

---

## 1. `src/instructions.md` — new sections

### 1.1 Add `## doc-authoring` section

Covers how to write a wiki page. Sourced from `agent-foundation/docs/authoring-guide.md`
and the wiki-specific frontmatter schema in `design.md`.

Content to include:

**Frontmatter fields — base (all pages)**

```yaml
title: "Short display title"          # matches H1 heading
summary: "One-line scope description" # not a restatement of title
read_when:
  - Specific condition completing "Read this when..."
  - 2–4 entries, actionable not vague
status: active                        # active | draft | deprecated | stub
last_updated: "YYYY-MM-DD"
```

**Frontmatter fields — wiki extensions**

```yaml
type: concept                         # concept | source-summary | query-result | contradiction
tags: [tag-a, tag-b]
sources: []                           # wiki-managed, do not set manually
confidence: high                      # high | medium | low
contradictions: []                    # wiki-managed, do not set manually
tldr: "One sentence."
```

**`summary` discipline**
- One sentence describing scope, not restating the title
- Bad: `"Skills authoring guide."` — restates title
- Good: `"How to author SKILL.md files with machine-readable YAML frontmatter."`

**`read_when` discipline**
- Each entry is a specific condition, not a topic label
- Bad: `- Skills` — too vague
- Good: `- Writing or reviewing a SKILL.md file`
- Aim for 2–4 entries

**Layout rules**
- Page with no assets → flat `.md` file
- Page with assets → bundle folder with `index.md` + assets beside it
- Asset references in bundle: `./asset.png` (short relative)
- Asset references to shared assets: `../../assets/diagrams/asset.png`

**What the LLM must not do**
- Do not write `sources:` or `contradictions:` — wiki manages these
- Do not write YAML frontmatter inside `body` — wiki generates it
- Do not use `description:` field — use `summary:`

### 1.2 Add `## enrichment-contract` section

Covers how to produce `enrichment.json`. Sourced from `design-evolution.md § 9`.

Content to include:

**When to produce enrichment.json**
- After reading an existing wiki page via `wiki_read` or MCP resource
- Goal: add semantic metadata to the page's frontmatter, not rewrite its body
- The body is already there — do not touch it

**Schema**

```json
{
  "source": "sources/switch-transformer-2021",
  "enrichments": [
    {
      "slug": "concepts/mixture-of-experts",
      "claims": [
        { "text": "...", "confidence": "high|medium|low", "section": "Results" }
      ],
      "concepts": ["scaling-laws", "transformer"],
      "tags": ["transformers", "scaling"],
      "read_when": ["Reasoning about MoE architecture"],
      "confidence": "high",
      "sources": ["sources/switch-transformer-2021"]
    }
  ],
  "query_results": [
    {
      "slug": "queries/my-question-2025",
      "title": "...",
      "tldr": "One sentence.",
      "body": "## Summary\n\n...",
      "tags": ["..."],
      "read_when": ["..."],
      "sources": ["concepts/mixture-of-experts"]
    }
  ],
  "contradictions": [
    {
      "title": "...",
      "claim_a": "...", "source_a": "sources/...",
      "claim_b": "...", "source_b": "sources/...",
      "dimension": "context|time|scale|methodology|open-dispute",
      "epistemic_value": "What this tension reveals.",
      "status": "active|resolved|under-analysis",
      "resolution": "Optional — only if resolved."
    }
  ]
}
```

**Field rules**
- `enrichments[].slug` — must be an existing page slug; wiki rejects unknown slugs
- `enrichments[].claims` — extract factual claims with confidence; omit if none
- `enrichments[].concepts` — slugs of related concept pages; omit if none
- `enrichments[].tags` — union with existing tags; never remove existing tags
- `enrichments[].read_when` — union with existing; never remove existing entries
- `enrichments[].confidence` — overall confidence in the source's claims
- `query_results[].body` — plain Markdown, no frontmatter block
- `contradictions[]` — only include if `wiki_context` was called first; omit if not

**What not to include**
- Do not include `body` in `enrichments` — body is never touched by enrichment
- Do not include `doc_type`, `title`, `language`, `key_quotes`, `data_gaps` — removed
- Do not include `suggested_pages` — replaced by `enrichments` + `query_results`

### 1.3 Update `## Analysis JSON contract` section

Replace the old `suggested_pages` schema with the new `enrichment.json` schema.
Point to `## enrichment-contract` for the full rules.

### 1.4 Update `## ingest-workflow` section

Reflect the new default: direct ingest first, enrichment optional.

```
1. wiki ingest <path>                    → pages exist, assets co-located
2. wiki_context(key concepts)            → find existing related pages
3. wiki_read(slug) for each relevant page → read content
4. Produce enrichment.json               → claims, concepts, contradictions
5. wiki_ingest(path, analysis: <json>)   → merge metadata into frontmatter
```

---

## 2. `wiki instruct` subcommand variants

`wiki instruct` currently prints the full guide. Add named variants so an LLM
can request only the section it needs:

```
wiki instruct                    # full guide (existing)
wiki instruct doc-authoring      # frontmatter schema + layout rules
wiki instruct enrichment         # enrichment.json contract
wiki instruct ingest             # ingest workflow
wiki instruct research           # research workflow
wiki instruct contradiction      # contradiction workflow
wiki instruct lint               # lint workflow
```

MCP tool update:

```rust
#[tool(description = "Print usage instructions for a specific workflow or topic")]
async fn wiki_instruct(
    &self,
    #[tool(param)] topic: Option<String>,  // None → full guide
) -> String { ... }
```

---

## 3. Implementation tasks

### `src/instructions.md`

- [ ] Add `## doc-authoring` section (§ 1.1)
- [ ] Add `## enrichment-contract` section (§ 1.2)
- [ ] Replace `## Analysis JSON contract` with updated schema (§ 1.3)
- [ ] Update `## ingest-workflow` to reflect new default (§ 1.4)
- [ ] Remove references to `suggested_pages`, `doc_type`, `action: create/update/append`

### `cli.rs`

- [ ] Add optional `[topic]` argument to `wiki instruct` subcommand
- [ ] Map topic strings to section anchors in `instructions.md`

### `server.rs`

- [ ] Add `topic: Option<String>` param to `wiki_instruct` MCP tool
- [ ] Return only the requested section when topic is provided

### `src/instructions.md` — section anchors

Each section in `instructions.md` must start with a level-2 heading that matches
the topic name exactly so the CLI/MCP can extract it by heading:

```markdown
## doc-authoring
## enrichment-contract
## ingest-workflow
## research-workflow
## contradiction-workflow
## lint-workflow
```

Extraction logic: find the heading, return everything until the next `##` heading
or end of file.

---

## 4. Tests

- [ ] `wiki instruct` with no args → returns full guide (existing test passes)
- [ ] `wiki instruct doc-authoring` → returns only the doc-authoring section
- [ ] `wiki instruct enrichment` → returns only the enrichment-contract section
- [ ] `wiki instruct unknown-topic` → error listing valid topics
- [ ] MCP `wiki_instruct(topic: None)` → full guide
- [ ] MCP `wiki_instruct(topic: Some("doc-authoring"))` → section only

---

## 5. Acceptance criteria

- [ ] An LLM calling `wiki instruct doc-authoring` receives enough information to
  write a correct wiki page frontmatter without reading any design doc
- [ ] An LLM calling `wiki instruct enrichment` receives enough information to
  produce a valid `enrichment.json` without reading `design-evolution.md`
- [ ] The old `suggested_pages` contract is no longer mentioned anywhere in
  `src/instructions.md`
- [ ] `wiki instruct` full guide is self-consistent with the new ingest model
