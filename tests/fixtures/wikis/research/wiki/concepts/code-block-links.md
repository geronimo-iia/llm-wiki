---
title: "Code Block Links"
type: concept
status: active
summary: "A page with TOML [[section]] headers inside fenced code blocks — for issue #127 regression."
tags: [test-fixture]
confidence: 0.5
last_updated: "2026-08-16"
read_when:
  - never — this page exists to test link extraction from code blocks
---

## Purpose

TOML `[[section]]` headers inside fenced code blocks must NOT be extracted
as wikilinks or produce broken-link findings.

See [[concepts/scaling-laws]] for a real link that must be extracted.

```toml
[[bench]]
name = "my_bench"
harness = false

[[pre-release-hooks]]
command = "cargo"
args = ["fmt", "--check"]
```

Also `[[not-a-link]]` in inline code must not be extracted.
