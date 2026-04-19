---
title: "Ingest Validation and Alias Resolution"
summary: "How frontmatter is validated against JSON Schema and aliases are resolved at indexing time."
status: draft
last_updated: "2025-07-18"
---

# Ingest Validation and Alias Resolution

For the ingest pipeline spec, see
[ingest-pipeline.md](../specifications/engine/ingest-pipeline.md).
For the type registry, see [type-registry.md](type-registry.md).
For the index schema, see [index-schema-building.md](index-schema-building.md).

## Validation

`ingest.rs` calls `registry.validate(frontmatter, strictness)` on
each page. The type registry looks up the page's type, falls back to
`default` if unknown, and validates against the compiled JSON Schema.
Strict mode rejects unknown types and validation errors. Loose mode
produces warnings.

## Alias Resolution

`build_document()` in `indexing.rs` resolves aliases before indexing:

```
1. page_type = fm["type"] or "page"
2. aliases = registry.aliases(page_type) or empty
3. for (field_name, value) in frontmatter:
   a. canonical = aliases.get(field_name) or field_name
   b. if IndexSchema.try_field(canonical) exists:
      - serialize value
      - add to document (text or keyword depending on field)
   c. else:
      - append serialized value to extra_text
4. Fixed fields:
   - slug, uri (always)
   - body = page.body + "\n" + extra_text
   - body_links = extracted [[wiki-links]]
```

A skill page with `name: "ingest"` gets indexed as `title: "ingest"`
— the alias mapping is applied transparently. The file on disk is
never modified. Unrecognized fields are appended to body text (BM25
searchable but not filterable).

## Value Serialization

Frontmatter values are `serde_yaml::Value`. They become strings for
tantivy:

| YAML type | Serialization |
|-----------|--------------|
| String | As-is |
| Sequence of strings | Space-joined for text fields, one `add_text` per entry for keyword fields |
| Boolean | "true" / "false" |
| Number | `.to_string()` |
| Mapping / nested | `serde_json::to_string()` |
| Null | Skip |

## Keyword vs Text for Arrays

`IndexSchema` holds `keyword_fields: HashSet<String>` populated
during construction. `build_document` checks membership to decide:

- **Text field** (tags, read_when): join values with space, single
  `add_text`
- **Keyword field** (sources, concepts): one `add_text` per entry
  (multi-valued exact match)
