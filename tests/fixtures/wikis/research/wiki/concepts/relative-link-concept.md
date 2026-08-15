---
title: "Relative Link Concept"
type: concept
status: active
summary: "Fixture page for testing CommonMark relative link normalization (issue #124)."
tags: [test-fixture]
last_updated: "2026-08-15"
read_when:
  - never — this page exists to test relative CommonMark link normalization
---

## Purpose

Tests that `./` relative CommonMark links are normalized to slugs before storage.

[sparse routing](./sparse-routing.md) is a valid relative link — resolves to
`concepts/sparse-routing` which exists and must NOT appear as broken.

[relative nonexistent](./relative-nonexistent.md) is a broken relative link —
resolves to `concepts/relative-nonexistent` which has no page and MUST appear
as a broken-link finding.
