# Jieba tokenizer registered unconditionally at every index open

## Decision

Register the jieba tokenizer on every `Index` instance at every open or create
site in `SpaceIndexManager`, regardless of the wiki's configured tokenizer.
Do not feature-gate `tantivy-jieba` behind a Cargo feature flag.

## Context

Tantivy tokenizers are referenced by name in the schema (e.g. `"jieba"`) but
registered in memory per `Index` instance. A tokenizer that was used when an
index was built must also be registered when the index is opened for reading or
searching — otherwise tantivy cannot locate the tokenizer and queries silently
return no results or panic.

`SpaceIndexManager` has five sites where an `Index` is opened or created:

1. `try_open` inside `open()`
2. `rebuild()` — `open_or_create` path
3. `rebuild()` — writer creation
4. `open_degraded()` recovery path
5. `try_open` inside `open_degraded()`

All five now call `register_custom_tokenizers(&index)` immediately after the
open. This is the only correct pattern: there is no safe place to register
"lazily" because the tokenizer must exist before any query is parsed.

### Alternatives considered

**Feature-gate with `--features cjk`.**  
The PR author offered this option to keep the default binary lean. It was
rejected for two reasons:

1. `jieba-rs` embeds its dictionary via `include-flate` (compressed at compile
   time, decompressed once at first use). Binary size impact is ~3 MB
   compressed; acceptable for a tool whose primary artifact is a search index
   that already occupies tens of megabytes on disk.
2. Feature flags add permanent maintenance burden: every CI matrix, every doc
   example, and every downstream embedder must opt in explicitly. The benefit
   (binary size) does not justify that cost at this project's scale.

**Register only when `config.tokenizer == Jieba`.**  
Rejected because it creates a footgun: a wiki migrated from jieba back to
`en_stem` would leave a jieba-encoded index on disk. The next process that
opens that index without registering jieba would fail to search it. Always
registering all supported tokenizers is safe, cheap (registration is a
`HashMap` insert), and avoids state-dependent bugs.

## Consequences

- `tantivy-jieba` is a hard dependency; it is always compiled and linked.
- The jieba dictionary (~3 MB compressed) is embedded in the binary.
- Users who want Chinese segmentation set `tokenizer = "jieba"` in `[index]`
  and run `wiki index rebuild`. No other configuration is required.
- Existing wikis using `en_stem`, `simple`, `raw`, or `default` are unaffected;
  the registered jieba tokenizer is unused unless the schema references it.
- If a future CJK tokenizer (e.g. lindera for Japanese/Korean) is added, it
  should follow the same pattern: register unconditionally in
  `register_custom_tokenizers`, add a variant to `config::Tokenizer`.
