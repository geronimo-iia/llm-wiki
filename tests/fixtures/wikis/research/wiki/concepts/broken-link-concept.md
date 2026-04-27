---
title: "Broken Link Concept"
type: concept
status: active
summary: "A page with a deliberately broken link for lint broken-link detection tests."
tags: [test-fixture]
confidence: 0.5
last_updated: "2026-03-01"
read_when:
  - never — this page exists to test lint rules
concepts:
  - concepts/does-not-exist
---

## Purpose

The `concepts` field references `concepts/does-not-exist`, which has no
corresponding page. This should appear in `wiki_lint(rules: "broken-link")`
findings as an Error.
