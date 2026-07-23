---
title: "Carry custom frontmatter fields in JSON export"
summary: "JSON export now includes a frontmatter object per page with fields not already surfaced at the top level."
status: accepted
date: "2026-07-23"
---

# Carry custom frontmatter fields in JSON export

## Decision

Add a `frontmatter` object to each JSON export entry. It contains every frontmatter field the page declares that is not already surfaced as a top-level field (`id`, `title`, `type`, `status`, `confidence`, `summary`). The object is omitted when a page has no extra fields. `llms-txt` and `llms-full` are unchanged.

## Context

`export --format json` previously emitted a fixed field set. Pages with custom type schemas (e.g. `decision` with `created`, `deciders`, `supersedes`; `source` with `year`, `authors`) exposed those fields only through per-page content reads. Consumers building summary tables or structured pipelines had to issue N+1 reads after the export.

## Rationale

- Downstream consumers (scripts, LLM pipelines) can build complete summary rows from the export alone
- Values preserve YAML typing: strings, numbers, arrays; YAML dates become strings
- Omit-when-empty keeps the output clean for standard pages with no extra fields
- JSON-only: llms-txt/llms-full are text formats where embedding extra fields has no natural representation

## Implementation

`load_bodies` gains a `with_frontmatter: bool` flag — only set for JSON format. When true, `crate::frontmatter::parse` is used instead of `strip_frontmatter`, and `remaining_frontmatter()` filters out the top-level fields before serialization. The `PageEntry` struct gains `frontmatter: serde_json::Map` with `skip_serializing_if = "Map::is_empty"`.
