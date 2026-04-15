---
title: "Session Bootstrap"
summary: "How the LLM orients itself at session start — schema.md injection, hub page reading, and the bootstrap sequence that makes every session start from the wiki's current state of knowledge."
read_when:
  - Understanding how the LLM gets oriented at session start
  - Implementing or extending the instruct workflows
  - Designing the MCP server initialize sequence
  - Understanding why hub pages matter for session continuity
status: draft
last_updated: "2025-07-15"
---

# Session Bootstrap

Every LLM session starts cold — no memory of prior conversations. The wiki
is the persistent context. Session bootstrap is the sequence that brings the
LLM from zero to oriented using the wiki itself.

---

## 1. The Problem

Without bootstrap, the LLM must discover the wiki's structure, conventions,
and current state through trial and error. With bootstrap, the LLM starts
every session already knowing:

- How this wiki is organized (schema.md)
- What the wiki contains (section index pages)
- What the current state of knowledge is (hub pages)
- What tools are available and how to use them (instructions)

The wiki replaces manually written session summaries. The crystallize workflow
feeds back into bootstrap — each crystallized session makes the next bootstrap
richer.

---

## 2. Bootstrap Sequence

Three layers, injected in order:

```
Layer 1: Instructions     ← how to use the wiki (tools, workflows, contracts)
Layer 2: Schema           ← how this wiki is organized (categories, conventions)
Layer 3: Orientation      ← what the wiki currently knows (hub pages)
```

### Layer 1 — Instructions (automatic)

`src/instructions.md` is injected at:
- MCP server start → `instructions` field on the server handler
- ACP `initialize` → system context
- Plugin slash commands → `wiki instruct <workflow>`

No LLM action required. This is handled by the engine.

### Layer 2 — Schema (automatic)

`schema.md` at the wiki root defines this wiki instance's conventions:
categories, ingest depth, lint rules, domain structure. Injected alongside
instructions at MCP/ACP start.

No LLM action required. This is handled by the engine.

### Layer 3 — Orientation (LLM-driven)

The LLM reads hub pages to understand the current state of knowledge. This
step is part of every workflow — not a separate command.

---

## 3. Orientation Step

Every instruct workflow begins with an orientation step. The LLM reads the
relevant section index pages and hub pages before doing any work.

### For ingest workflows

```
1. wiki_read(schema.md)                          ← know the conventions
2. wiki_search("<topic of source>", no_excerpt)   ← find related pages
3. wiki_read(<most relevant hub page>)            ← understand current state
4. ... write pages into wiki tree, then wiki_ingest
```

### For research workflows

```
1. wiki_search("<question>", no_excerpt)           ← find relevant pages
2. wiki_read(<top results>)                        ← read the knowledge
3. ... synthesize answer
```

### For crystallize workflows

```
1. wiki_search("<topic>", no_excerpt)              ← find existing home
2. wiki_read(<candidate hub page>)                 ← check if update fits
3. ... write page into wiki tree, then wiki_ingest
```

### For lint workflows

```
1. wiki_lint()                                     ← get structural report
2. wiki_read(<orphan pages>)                       ← understand what's orphaned
3. ... address findings
```

---

## 4. Hub Pages

A hub page is any section index or concept page that serves as a navigation
entry point for a domain. Hub pages are the most valuable bootstrap targets
because they summarize an entire area of knowledge.

Hub pages are not a special type — they are regular pages that happen to be
well-connected and comprehensive. The LLM identifies them by:

- Section `index.md` pages (type: `section`)
- Pages with high in-degree in the concept graph
- Pages referenced by many other pages in `sources` or `concepts` frontmatter

The crystallize workflow naturally produces and enriches hub pages — each
session's distilled knowledge updates the relevant hub, making the next
session's bootstrap richer.

---

## 5. Instruct Integration

The orientation step is documented in each workflow section of
`src/instructions.md`. It is not a separate workflow — it is the first step
of every workflow.

Addition to the preamble of `src/instructions.md`:

```markdown
## Session orientation

At the start of any workflow, orient yourself to the wiki's current state:

1. Read `schema.md` if you haven't already (injected automatically via MCP —
   skip if you already have it in context).
2. Search for the topic you're about to work on: `wiki_search("<topic>")`.
3. Read the most relevant hub page or section index to understand the current
   state of knowledge before making changes.

The wiki is the persistent context across sessions. Start from what it knows,
not from scratch.
```

---

## 6. MCP Server Integration

The MCP server injects both `src/instructions.md` and the active wiki's
`schema.md` at session start. Because `schema.md` is per-wiki and loaded at
runtime, the injection is a two-step operation:

1. `src/instructions.md` is embedded at compile time via `include_str!()`
2. At server startup, `schema.md` is read from the default wiki root and
   appended to the instructions string before being passed to the MCP handler

When multiple wikis are mounted, the `schema.md` of `global.default_wiki` is
used for the session-level injection. Per-wiki schema is available to the LLM
via `wiki_read("schema.md", wiki: "<name>")` for non-default wikis.

---

## 7. The Bootstrap Loop

Crystallize and bootstrap form a compounding loop:

```
Session N:
  bootstrap → read hub page → work → crystallize → update hub page → commit

Session N+1:
  bootstrap → read updated hub page → richer starting context → ...
```

Each session starts from a richer baseline than the last. The wiki is the
accumulator. The LLM is stateless — the wiki is not.
