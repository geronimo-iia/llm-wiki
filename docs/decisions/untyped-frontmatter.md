# Untyped Frontmatter Parsing

## Decision

The frontmatter parser returns `BTreeMap<String, serde_yaml::Value>`,
not a fixed Rust struct. All type-aware logic (validation, aliases,
required fields) lives in the type registry, not the parser.

## Context

The original parser used a `PageFrontmatter` struct with all known
fields. This worked when every page had the same schema. With the
dynamic type system, a skill page has `name`/`description`, a custom
type has arbitrary fields — a fixed struct can't represent them all.

## Alternatives Considered

| Approach                                                      | Why not                                                      |
| ------------------------------------------------------------- | ------------------------------------------------------------ |
| Fixed struct with `HashMap<String, Value>` for extras         | Two-tier access — some fields typed, some dynamic. Confusing |
| Enum per type (`ConceptFrontmatter`, `SkillFrontmatter`, ...) | Doesn't scale to custom types                                |
| Trait object per type                                         | Over-engineered for what's essentially a key-value map       |

## How It Works

The parser does two things:
1. Split the file into YAML frontmatter and body
2. Parse the YAML into `BTreeMap<String, Value>`

That's it. The parser doesn't know about types. The `SpaceTypeRegistry`
takes the map and:
- Validates it against the type's JSON Schema
- Resolves aliases (`name` → `title`)
- Extracts edge declarations for the graph

## Consequences

- The parser is simple — ~15 lines of split logic + `serde_yaml`
- Adding a new type requires no parser changes
- Typed access to common fields (`title`, `type`) via convenience
  methods on the parsed result
- Validation errors come from JSON Schema, not from serde
  deserialization failures
