# Decision: yaml_to_text / yaml_to_strings — Leave As-Is

## Context

Two private functions in `index_manager.rs` traverse `serde_yaml::Value`
with similar patterns. Design review flagged as "Minor."

→ [analysis prompt](../prompts/yaml-value-extraction.md)

## Decision

**Leave as-is.**

- Both are <20 lines, private, adjacent in the same file
- `yaml_to_strings` already calls `yaml_to_text` as its fallback
- `Sequence` handling intentionally differs: join with space (text
  fields) vs individual items (keyword fields)
- Unifying would require a mode parameter that obscures the intent
