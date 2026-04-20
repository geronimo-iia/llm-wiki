# LLM Wiki — The Crystallize Pattern

from https://github.com/vanillaflava/llm-wiki-claude-skills

The most original contribution of this implementation. Karpathy noted that "good answers can be filed back into the wiki." This project turns that observation into a first-class operation.

## Core Idea

Chats accumulate context that is costly to carry, hard to search, and increasingly dominated by superseded content. Crystallize compresses the durable signal — decisions, findings, patterns, open questions — into a structured wiki page that orients any future session faster than re-reading the thread.

The chat is the scaffolding. The wiki page is the artefact.

## What to Keep vs Discard

**Keep:** decisions made, patterns established, lessons learned, open questions, current understanding, key findings, agreed frameworks.

**Discard:** exploratory back-and-forth, dead ends, process chat, superseded drafts, corrections already incorporated.

## Three Levels of Crystallization

| Level | When | Closing posture |
|---|---|---|
| Single session | End of a working session; context still valuable | Pause — continue or return |
| Topic thread | After several sessions; thread getting heavy | Recommend fresh start |
| Whole chat | Thread exhausted or explicitly being archived | Strong close — wiki page is the successor |

## The Session Bootstrap Loop

This is where crystallize compounds:

1. Work in a chat session → accumulate decisions, findings, patterns
2. Crystallize → distil into a wiki page (or update an existing hub page)
3. Start fresh session → read the wiki page first for orientation
4. The wiki page IS the persistent context across sessions

Domain home pages serve as structured session bootstraps. Previously that context lived in manually written summaries. Now the wiki itself is the persistent context.

## Workflow

1. Prefer updating existing pages over creating new ones — check index.md first
2. If multiple candidates exist, ask the user which page to enrich
3. Update frontmatter version, date, and changes fields
4. Update Overview.md only for significant knowledge shifts (major decisions, completed research phases, new domains)
5. Choose closing posture based on stated intent + thread weight

## Page Template (new pages only)

```markdown
---
title: Topic — Current State
version: 1.0
date: YYYY-MM-DD
changes: Crystallized from [source chat / session description]
---

## What Was Established
## Key Decisions
## Current Understanding
## Open Questions
## Related Pages
```

## Why This Matters

The author's key insight: "I had already been prompting summaries of heavy chats, and cycling them out for a fresh instance. Now distilling a long working session into a wiki page became the primary way my projects accumulate knowledge from conversations."

This turns the LLM from a stateless tool into a system with persistent, compounding memory — mediated through the filesystem rather than through any proprietary memory feature.
