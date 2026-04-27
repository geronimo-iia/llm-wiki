---
title: "Skills Validation Guide — v0.4.0"
summary: "Interactive session guide for validating all llm-wiki-skills against a live engine."
---

# Skills Validation Guide — v0.4.0

Run these scenarios in Claude with the llm-wiki MCP plugin active.
Each scenario specifies: the skill to invoke, the test document or context to use,
and the expected behavior checklist.

## Installation

**1. Install the engine binary:**

```bash
cargo install llm-wiki-engine
# or download a pre-built binary from GitHub releases
```

**2. Install the Claude plugin:**

```bash
# From the Claude marketplace
claude plugin add llm-wiki-skills

# Or directly from GitHub
claude plugin add https://github.com/geronimo-iia/llm-wiki-skills

# Or from a local clone (development)
claude --plugin-dir ./llm-wiki-skills
```

**3. Start the engine:**

```bash
llm-wiki serve
```

---

## Setup required before running scenarios

**1. Create and register both wikis:**

```bash
# research wiki
llm-wiki spaces create tests/fixtures/wikis/research --name research
# notes wiki (cross-wiki target)
llm-wiki spaces create tests/fixtures/wikis/notes --name notes
# set research as default
llm-wiki spaces set-default research
```

**2. Build the index:**

```bash
llm-wiki index rebuild --wiki research
llm-wiki index rebuild --wiki notes
```

**3. Copy inbox documents:**

```bash
cp tests/fixtures/inbox/* tests/fixtures/wikis/research/inbox/
```

**4. Start the engine** (if not already running):

```bash
llm-wiki serve
```

---

## Scenario 1 — Setup skill

**Prompt:**
```
/setup
```

**Follow-up if it asks for a path:**
```
tests/fixtures/wikis/research
```

**Expected:**
- [ ] Asks for or confirms wiki root path
- [ ] Runs `wiki_spaces_create` or confirms existing space
- [ ] Reports wiki name, path, and default status
- [ ] Does not error if space already exists

---

## Scenario 2 — Ingest skill (basic)

**Prompt:**
```
/ingest inbox/01-paper-switch-transformer.md
```

**When the analysis plan is presented, confirm with:**
```
looks good, proceed
```

**If it asks about the contradiction with concepts/sparse-routing:**
```
flag it as an open question, don't resolve it
```

**Expected:**
- [ ] Calls `wiki_ingest(path: "inbox/01-paper-switch-transformer.md", dry_run: true)` first
- [ ] Reads the file and classifies it as `paper`
- [ ] Calls `wiki_list(format: "llms")` for orientation (first file in session)
- [ ] Searches for existing pages on "Switch Transformer" and "mixture of experts"
- [ ] Finds `concepts/mixture-of-experts` and `sources/mixtral-paper` as integration points
- [ ] **Analysis step (imp-11):** produces an ingest plan with items like:
  - Switch Transformer — new source page
  - Top-1 routing — extends or new concept
  - Contradiction with `concepts/sparse-routing` (compute cost claim)
- [ ] Presents plan and waits for confirmation before writing
- [ ] After confirmation: creates `sources/switch-transformer-2021` page
- [ ] Sets `confidence: 0.5` on new page (single source, unreviewed)
- [ ] Validates via `wiki_ingest(path: "wiki/sources/...", dry_run: true)` before committing
- [ ] Moves `inbox/01-paper-switch-transformer.md` → `raw/`
- [ ] Reports: pages created, pages updated, files processed

---

## Scenario 3 — Ingest skill (redaction)

**Prompt:**
```
/ingest inbox/03-note-with-secrets.md
```

**If it does not automatically suggest redaction, follow up with:**
```
this file contains API keys and personal data — please ingest it with redaction enabled
```

**When the analysis plan is presented, confirm with:**
```
proceed with redaction
```

**Expected:**
- [ ] Classifies source as `note`
- [ ] Because it's raw notes/transcript, suggests or applies `redact: true`
- [ ] After ingest, any created page does NOT contain `sk-ant-api03` or `sk-proj-` strings
- [ ] Report includes `redacted` field listing matched patterns and line numbers
- [ ] At least: `anthropic-key`, `openai-key`, `bearer-token`, `email` patterns triggered

---

## Scenario 4 — Ingest skill (cross-wiki reference)

**Prompt:**
```
/ingest inbox/04-note-cross-wiki.md
```

**When the analysis plan is presented, confirm with:**
```
proceed — preserve the wiki:// link as-is
```

**Expected:**
- [ ] Classifies as `note`
- [ ] Detects `wiki://notes/concepts/attention-mechanism` in body
- [ ] Attempts `wiki_content_read(uri: "wiki://notes/concepts/attention-mechanism")` to verify target
- [ ] Creates page with `wiki://notes/concepts/attention-mechanism` preserved in body or concepts field
- [ ] Does NOT treat the URI as a broken link

---

## Scenario 5 — Crystallize skill

**Send this context first to simulate a working session:**
```
I've been reading about Mixture of Experts models today. Key things I learned:
- Mixtral uses top-2 routing with 8 experts, only 12.9B active params out of 46.7B total
- The Switch Transformer uses top-1 routing which is simpler but may cause expert collapse
- There's a debate about whether MoE is truly compute-efficient: FLOPs are lower but memory bandwidth is the real bottleneck in practice
- Sparse routing requires load balancing losses or experts collapse to always routing the same few
```

**Then crystallize:**
```
/crystallize
```

**When the extraction plan is presented, confirm with:**
```
looks good, proceed with all items
```

**If it asks about confidence for the memory bandwidth finding:**
```
that's a finding from a single article, set it to 0.6
```

**Expected:**
- [ ] Calls `wiki_list(format: "llms")` for orientation
- [ ] **Analysis step:** enumerates durable knowledge items from the session
  - Format: What / Type / Action / Confidence per item
- [ ] Presents extraction plan and waits for confirmation
- [ ] After confirmation: writes only confirmed items
- [ ] Sets `confidence` based on calibration table (decisions ~0.8, findings ~0.6, speculation ~0.3)
- [ ] Runs `wiki_lint(rules: "broken-link,orphan")` after writing
- [ ] Reports any lint findings introduced by new pages

---

## Scenario 6 — Research skill

**Prompt:**
```
/research What is the compute cost of Mixture of Experts models?
```

**Follow-up to verify backlinks are used:**
```
which pages link to concepts/sparse-routing?
```

**Follow-up to verify contradiction surfacing:**
```
are there any conflicting claims about this in the wiki?
```

**Expected:**
- [ ] Calls `wiki_search` with relevant queries
- [ ] Reads `concepts/mixture-of-experts`, `concepts/sparse-routing`, `concepts/compute-efficiency`
- [ ] Uses `wiki_content_read(backlinks: true)` on key pages to find related sources
- [ ] Identifies the contradiction: `sparse-routing` claims O(k/n), `compute-efficiency` draft claims O(n)
- [ ] Reports both claims with their confidence values
- [ ] Does not present the contradiction as resolved

---

## Scenario 7 — Lint skill

**Prompt:**
```
/lint
```

**Follow-up to verify fix guidance:**
```
how do I fix the broken link finding?
```

**Follow-up to verify orphan handling:**
```
what should I do with the orphan page?
```

**Expected:**
- [ ] Calls `wiki_list(format: "llms")` for structural orientation
- [ ] Calls `wiki_lint()` for all rules
- [ ] Reports findings grouped by severity (Errors first)
- [ ] `broken-link` Error: `concepts/broken-link-concept` → `concepts/does-not-exist`
- [ ] `orphan` finding: `concepts/orphan-concept`
- [ ] Provides fix guidance for each finding type
- [ ] For `orphan`: suggests running `wiki_suggest` to find link candidates
- [ ] For `broken-link`: suggests correcting or removing the reference

---

## Scenario 8 — Graph skill

**Prompt:**
```
/graph
```

**Follow-up to verify community insights:**
```
which pages are isolated from the main graph?
```

**Follow-up to verify cross-wiki graph:**
```
show me the cross-wiki graph including the notes wiki
```

**Follow-up to verify suggestions for isolated pages:**
```
suggest links for the orphan page
```

**Expected:**
- [ ] Calls `wiki_graph(format: "llms")` as primary interpretation
- [ ] Calls `wiki_stats()` and reads `communities` field
- [ ] Reports cluster count, largest/smallest cluster sizes
- [ ] Lists `isolated` slugs (should include `concepts/orphan-concept`)
- [ ] Suggests `wiki_suggest` for isolated pages
- [ ] Optionally renders Mermaid for visual inspection

**Expected after cross-wiki follow-up:**
- [ ] Calls `wiki_graph(cross_wiki: true)`
- [ ] Output includes `notes/concepts/attention-mechanism` as a node
- [ ] Describes the cross-wiki edge from research to notes

---

## Scenario 9 — Content skill

**Prompt:**
```
using the content skill, read concepts/compute-efficiency with backlinks, then update its confidence to 0.4 and add a note about the open question on compute cost
```

**Follow-up to verify accumulation contract:**
```
make sure you keep the existing claims — only add the new note, don't replace anything
```

**Follow-up to verify commit:**
```
commit the change
```

**Expected:**
- [ ] Calls `wiki_content_read(slug: "concepts/compute-efficiency", backlinks: true)`
- [ ] Shows current content + backlinks (should show `sources/mixtral-paper` links here)
- [ ] Reads current page before writing (accumulation contract)
- [ ] Preserves existing `claims[]` — does not drop them
- [ ] Updates `confidence` to 0.4
- [ ] Adds note in body about the O(k/n) vs O(n) open question
- [ ] Calls `wiki_ingest` to validate and commit

---

## Scenario 10 — Review skill

**Prompt:**
```
/review
```

**When it presents the first item (broken-link-concept), respond:**
```
update — remove the broken concepts link, it was a mistake
```

**When it presents the orphan page:**
```
defer — I'll link it later
```

**When it presents compute-efficiency:**
```
update — raise confidence to 0.6, the O(k/n) claim is probably correct for FLOPs
```

**To end the session early:**
```
stop here and give me the summary
```

**Expected:**
- [ ] Calls `wiki_lint()` → Sources 1 (errors + warnings)
- [ ] Calls `wiki_list(status: "draft")` → Source 2
- [ ] Calls `wiki_list(status: "active")` → Source 3 (filters confidence < 0.4)
- [ ] Builds priority queue: `concepts/broken-link-concept` (Error), `concepts/orphan-concept` (draft/Warning), `concepts/compute-efficiency` (draft, confidence 0.5)
- [ ] Presents first item with context (what triggered flag, backlinks, recent history)
- [ ] Offers 5 decision options: Promote / Update / Resolve contradiction / Defer / Flag for deletion
- [ ] Processes decisions one at a time, commits after each
- [ ] Reports final summary: processed count by outcome + remaining queue

---

## Scenario 11 — Schema skill

**Prompt:**
```
/schema show concept
```

**Follow-up to verify claims confidence type:**
```
what type is claims[].confidence — is it a float or a string?
```

**Follow-up to verify template:**
```
show me the template for a new concept page
```

**Expected:**
- [ ] Calls `wiki_schema(action: "show", type: "concept")`
- [ ] Shows required fields (`title`, `type`, `read_when`)
- [ ] Shows optional fields with types
- [ ] Shows graph edge declarations if present
- [ ] Reports `claims[].confidence` as float 0.0–1.0 (not string enum)

---

## Scenario 12 — Spaces skill

**Prompt:**
```
/spaces
```

**Follow-up to verify switching default:**
```
set notes as the default wiki
```

**Follow-up to verify it can switch back:**
```
set research as the default wiki again
```

**Expected:**
- [ ] Calls `wiki_spaces_list()`
- [ ] Shows both `research` and `notes` wikis
- [ ] Shows which is the default
- [ ] Reports wiki paths

---

## Pass/Fail criteria

| Scenario | Core feature tested |
|---|---|
| 1 | Setup, space registration |
| 2 | Ingest two-step (imp-11), confidence default 0.5 |
| 3 | Redaction (imp-06) |
| 4 | Cross-wiki links in ingest (imp-10) |
| 5 | Crystallize two-step + confidence calibration (imp-07) |
| 6 | Research + backlinks (imp-03) |
| 7 | Lint skill + wiki_lint (imp-04) |
| 8 | Graph + communities (imp-08) + cross-wiki graph (imp-10) |
| 9 | Content + backlinks (imp-03) + accumulation contract |
| 10 | Review skill (imp-12) |
| 11 | Schema + claims confidence float (imp-01b) |
| 12 | Spaces |

A scenario passes if all checkboxes are satisfied.
A scenario fails if any checkbox is not met — note the specific failure for the bug report.
