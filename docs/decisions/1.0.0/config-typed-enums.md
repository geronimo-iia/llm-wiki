# Typed config enums — TypeStrictness, Tokenizer, GraphFormat, GraphRenderFormat

## Decision

Replace the five `String`-typed config fields that acted as discriminated enums
with actual Rust enums. Introduce a second, runtime-only enum (`GraphRenderFormat`)
for graph output that adds the `Summary` variant which cannot live in the stored
config.

Affected types in `src/config.rs`:
- `TypeStrictness` — `Loose` (default) | `Strict`; `Copy`
- `Tokenizer` — `EnStem` (default) | `Raw` | `Simple` | `Default`; `Copy`
- `GraphFormat` — `Mermaid` (default) | `Dot` | `Llms` | `Json`; `Clone`

Affected type in `src/ops/graph.rs`:
- `GraphRenderFormat` — `Mermaid` | `Dot` | `Llms` | `Json` | `Summary`; `Copy`

`From<GraphFormat> for GraphRenderFormat` converts the stored default to the
runtime type. Internal APIs (`validate`, `SchemaBuilder::new`, `GraphParams`)
take the typed enum; the external Tantivy boundary (`set_tokenizer`) still
receives `tokenizer.as_str()`.

## Context

Before this change, five config fields were stored as `String` and threaded
through internal APIs as `&str`. Every call site carried a `.as_str()` call, and
every parse site carried a `match s { "loose" => ..., "strict" => ..., other =>
bail!() }` block copied by hand. Unknown values were caught at runtime, not
compile time. Adding a new variant required updating the parse match, the
internal API signatures, and all call sites independently, with no compiler
assistance.

The five string fields were:
- `ValidationConfig.type_strictness: String` — `"loose"` or `"strict"`
- `IndexConfig.tokenizer: String` — `"en_stem"`, `"raw"`, `"simple"`, `"default"`
- `GraphConfig.format: String` — `"mermaid"`, `"dot"`, `"llms"`, `"json"`
- `GraphParams.format: Option<&str>` — same four values plus `"summary"`
- `TypeRegistry.validate(strictness: &str)` — threaded `&str` into a match

## Why `TypeStrictness` and `Tokenizer` are `Copy` but `GraphFormat` is not

`TypeStrictness` and `Tokenizer` are function parameters: every ingest
operation passes them into `validate()` and `SchemaBuilder::new()`. Making them
`Copy` matches the ergonomics of `&str` — pass by value, no `.clone()` at call
sites, no lifetime to manage.

`GraphFormat` is embedded in `GraphConfig`, which is part of `ResolvedConfig`
and `GlobalConfig`. Those structs derive `Clone` (not `Copy`) because they
contain `String` fields. Making `GraphFormat` `Copy` is possible but delivers
no benefit — it is always accessed through a reference inside the config structs
and only needs to be owned once (when converting to `GraphRenderFormat`).

## Why `GraphRenderFormat` is a separate type

`Summary` is a display-only aggregate mode: it returns graph metrics and
community statistics without rendering any graph. It has no meaning as a stored
default in `wiki.toml`. Putting it in `GraphFormat` would allow
`graph.format = "summary"` in config — semantically wrong, since the summary
mode is a per-request display choice, not a persistent rendering preference.

Keeping `GraphFormat` as the config type and `GraphRenderFormat` as the runtime
type enforces this invariant at the type level. The four shared variants map
one-to-one; `From<GraphFormat> for GraphRenderFormat` makes the conversion
exhaustive — adding a new config variant without handling it in the `From` impl
is a compile error.

## Why `as_str()` is retained on all four enums

Three external boundaries take string values:
- Tantivy `set_tokenizer(&str)` — the Tantivy API is not under our control.
- `wrap_graph_md(rendered, format: &str, filter)` — the format string is
  embedded in a markdown code-fence label; `&'static str` keeps the call
  zero-allocation.
- `config set/get` CLI round-trips through string keys.

`as_str()` on each enum provides a single definition of the canonical string
form. `Display` delegates to `as_str()`. `FromStr` parses canonical strings and
produces a clear error message naming the allowed values.

## Alternatives considered

**Single enum with `#[serde(skip)]` on `Summary`.** Would avoid the parallel
type. Rejected — `#[serde(skip)]` silently omits the variant from
deserialization, not serialization; a `GraphFormat::Summary` would serialize
as nothing and round-trip incorrectly. Separate types are explicit about the
boundary.

**Keep `GraphFormat` as `Copy`.** Viable but unnecessary. `Clone` is sufficient
for all call sites, and the structs it lives in already require `Clone`.

**Remove `as_str()`, use `Display` everywhere.** `Display` allocates a `String`
when used at Tantivy call sites. `as_str()` returns `&'static str` — zero
allocation, directly passable as `&str`.

## Consequences

- Exhaustive match on all enum variants; the compiler catches unhandled cases
  when a variant is added.
- Invalid config values (e.g. `graph.format = "svg"`) are rejected at parse
  time with a message listing allowed values, not at first use.
- `TypeStrictness` and `Tokenizer` pass by value throughout internal APIs —
  no `.clone()` or lifetime management at call sites.
- `GraphRenderFormat::Summary` is unreachable from stored config; the
  `From<GraphFormat>` conversion is the only bridge from config to runtime.
- Internal `as_str()` calls remain at the two Tantivy and markdown boundaries;
  all other internal code uses the enum directly.
