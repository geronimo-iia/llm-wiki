---
title: "Crystallize"
summary: "Instruct workflow for distilling a chat session into a wiki page — the LLM writes directly into the wiki tree and ingests it."
read_when:
  - Understanding how ephemeral chat knowledge enters the wiki
  - Implementing the crystallize instruct workflow
  - Designing the plugin slash command for crystallize
status: draft
last_updated: "2025-07-15"
---

# Crystallize

Crystallize is an instruct workflow, not a separate engine command. The LLM
writes directly into the wiki tree and runs `llm-wiki ingest` to validate
and index. The workflow guides the LLM on *what to extract* from a
session and *where to put it*.

---

## 1. The Problem

Ingest handles file-derived knowledge: a paper, an article, a transcript.
But valuable knowledge also emerges from conversations — decisions reached,
patterns discovered, questions resolved, designs settled. Without crystallize,
this knowledge disappears when the chat ends or the context window fills.

Crystallize closes the loop: the wiki compounds from both sources and
conversations.

```
Sources  → LLM reads, writes pages into wiki tree  → llm-wiki ingest → validated pages
Chats    → crystallize workflow (LLM writes pages)  → llm-wiki ingest → validated pages
```

---

## 2. What Crystallize Captures

**Keep:** decisions made, patterns established, lessons learned, open questions,
current understanding, key findings, agreed frameworks, design rationale.

**Discard:** exploratory back-and-forth, dead ends, process chat, superseded
drafts, corrections already incorporated.

The output is always significantly shorter and more structured than the input.
Crystallize distils — it does not transcribe.

---

## 3. Workflow

### Step 1 — Search for an existing home

Before creating anything new, search the wiki for a page that already covers
this topic:

```
wiki_search("<topic>")           → Vec<PageRef>
wiki_read(<candidate slug>)      → existing page content
```

If a concept page, section index, or prior query-result already covers this
ground, prefer updating it over creating a new page.

### Step 2 — Write the page

The LLM writes a complete Markdown file directly into the wiki tree using
`wiki_write`, following the [frontmatter authoring guide](../core/frontmatter-authoring.md).

**For a new page** (no existing home found):

```markdown
---
title: "MoE Routing — Design Decision"
summary: "Expert-choice routing selected for inference pipeline."
tldr: "Expert-choice gives best quality/efficiency tradeoff above 10B."
read_when:
  - "Reviewing MoE routing decisions"
  - "Understanding why expert-choice was selected"
status: active
last_updated: "2025-07-15"
type: query-result
tags: [moe, routing, inference]
sources: [sources/switch-transformer-2021]
concepts: [concepts/mixture-of-experts]
confidence: medium
---

## Summary

Settled on expert-choice routing for the inference pipeline...

## Decisions

- Expert-choice routing selected over top-k for quality/efficiency tradeoff
- Hash routing reserved for batch-only workloads

## Findings

- MoE compute gains are phase-dependent — diminish beyond 100B params
- Expert-choice adds 2% quality at 3.4x throughput

## Open Questions

- Does expert-choice routing degrade under mixed-precision inference?
- What is the memory overhead of maintaining expert affinity tables?
```

**For an update** (existing page found):

1. `wiki_read(<slug>)` — get current content
2. Merge new knowledge into existing content — preserve existing frontmatter
   values, add new tags/sources/claims, update body sections
3. Write the complete updated file via `wiki_write`

### Step 3 — Ingest

```
wiki_ingest("queries/moe-routing-decision.md")                    # new page
wiki_ingest("concepts/mixture-of-experts.md")                     # update
```

### Step 4 — Verify

```
wiki_read(<slug>)   → confirm the knowledge was captured correctly
```

---

## 4. Suggested Body Structure

The crystallize workflow suggests this body structure, but the LLM adapts
it to the content:

| Section | When to include |
|---------|-----------------|
| Summary | Always — 2–4 sentences of what was established |
| Decisions | When decisions were made |
| Findings | When new knowledge was discovered |
| Current Understanding | When the session advanced understanding of a topic |
| Open Questions | When questions remain unresolved |
| Related Pages | When connections to other wiki pages are worth noting |

Not every section is needed. A lightweight session might only warrant a
Summary and Findings. A heavy design session might need all sections.

---

## 5. When to Crystallize

Use crystallize liberally — any time something meaningful has happened:

| Signal | Action |
|--------|--------|
| A decision was reached | Crystallize the decision and rationale |
| New knowledge was absorbed | Crystallize findings into a concept page |
| A question was resolved | Update the page that had the open question |
| A design was settled | Crystallize into a query-result or concept page |
| The chat is getting heavy | Crystallize before context degrades |
| Closing a long thread | Crystallize everything worth keeping |

---

## 6. Instruct Integration

Added to `src/instructions.md` as `## crystallize`:

```markdown
## crystallize

Distil the current session into a wiki page. Use when a decision has been
reached, a pattern discovered, a question resolved, or a design settled.

1. **Search for an existing home** — `wiki_search("<topic>")`. If a page
   already covers this ground, prefer updating it over creating a new page.

2. **If updating:** `wiki_read(<slug>)` to get current content. Merge new
   knowledge — preserve existing frontmatter values, add to lists, update
   body sections.

3. **Write directly into the wiki tree** via `wiki_write(path, content)`.
   Follow the `## frontmatter` guide. Use `type: query-result` for new
   crystallizations. Structure the body with: Summary, Decisions, Findings,
   Open Questions as appropriate.

4. **Ingest:** `wiki_ingest(path)` to validate and index.
5. **Commit:** `wiki_commit(slugs)` to commit, or skip if `auto_commit` is on.
6. **Verify:** `wiki_read(<slug>)` to confirm.

5. **Verify:** `wiki_read(<slug>)` to confirm.
```

---

## 7. Plugin Slash Command

Added to `.claude-plugin/commands/`:

```
/llm-wiki:crystallize
```

Fetches workflow instructions from `llm-wiki instruct crystallize` and guides the
LLM through the crystallize flow.

---

## 8. Why Not a Separate Command?

The previous design had `llm-wiki crystallize` as a separate CLI command with a
`CrystallizeRequest` JSON schema. This is removed because:

- The LLM writes directly into the wiki tree
- A separate JSON schema adds complexity for no gain
- The value of crystallize is the *workflow guidance* (what to extract, where
  to put it), not a distinct engine operation
- `llm-wiki ingest` validates and indexes whatever is on disk

Crystallize is a workflow, not a tool. The tools are `wiki_write` + `wiki_ingest`.

---

## 9. The Bootstrap Loop

Crystallize and session bootstrap form a compounding loop:

```
Session N:
  bootstrap → read hub page → work → crystallize → ingest → commit

Session N+1:
  bootstrap → read updated hub page → richer starting context → ...
```

Each session starts from a richer baseline than the last. The wiki is the
accumulator. The LLM is stateless — the wiki is not.
