# Testing

End-to-end validation for the llm-wiki CLI and Claude plugin.

## Layout

```
docs/testing/
  README.md             ← this file
  validate-skills.md    ← interactive scenarios for Claude plugin validation
  scripts/
    validate-engine.sh  ← CLI end-to-end validation script

tests/fixtures/         ← static test data (wiki spaces + inbox documents)
  wikis/
    research/           ← primary test wiki (MoE / transformer domain)
      wiki.toml         ← sets min_nodes_for_communities=5 for community detection tests
      wiki/
        concepts/       ← 6 concept pages (includes orphan + broken-link fixtures)
        sources/        ← 1 source page
        inbox/          ← empty; script copies inbox/ files here before testing
        raw/            ← populated by ingest on success
    notes/              ← second wiki for cross-wiki tests
      wiki.toml
      wiki/
        concepts/       ← 1 concept page (attention-mechanism, cross-wiki target)
  inbox/
    01-paper-switch-transformer.md  ← rich paper; tests ingest + contradiction detection
    02-article-moe-efficiency.md    ← article; tests claim contradiction with sparse-routing
    03-note-with-secrets.md         ← contains fake API keys; tests redaction (imp-06)
    04-note-cross-wiki.md           ← contains [[wiki://notes/...]]; tests cross-wiki (imp-10)
    05-data-benchmark-scores.csv    ← CSV; tests data source type classification
```

## Deliberate fixtures for lint rules

| Page | Rule triggered | Why |
|---|---|---|
| `concepts/orphan-concept.md` | `orphan` | No inbound or outbound links |
| `concepts/broken-link-concept.md` | `broken-link` | `concepts` field references `concepts/does-not-exist` |
| `concepts/compute-efficiency.md` | `stale` (over time) | Low confidence draft |

## Deliberate contradictions

- `concepts/sparse-routing` claims compute cost is O(k/n)
- `concepts/compute-efficiency` draft claims compute cost is O(n)
- `02-article-moe-efficiency.md` also argues the O(k/n) claim is misleading

These contradictions are intentional for testing the ingest analysis step (imp-11)
and the review skill (imp-12).

## Usage

**Engine script:**
```bash
LLM_WIKI_BIN=./target/release/llm-wiki ./docs/testing/scripts/validate-engine.sh
```

**Skills guide:**
Open `docs/testing/validate-skills.md` and run each scenario in Claude with the plugin active.
Both wikis must be registered before starting.
