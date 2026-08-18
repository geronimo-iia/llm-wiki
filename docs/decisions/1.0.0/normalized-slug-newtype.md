# `NormalizedSlug` newtype for compile-time slug invariant

## Decision

Add `NormalizedSlug(String)` to `src/slug.rs`. Change `PageRef.slug` and
`PageSummary.slug` from `String` to `NormalizedSlug`. Provide
`Slug::normalize() -> NormalizedSlug` as the only public construction path.
Add `pub(crate) NormalizedSlug::from_normalized(String)` as an internal
bypass for index reads where the stored value is already known to be
normalized.

## Context

Slug normalisation (lowercase, canonical form) was convention-enforced only.
A raw `String` slug from user input and a `Slug`-derived slug from the index
could be compared with `==` without the compiler catching the mismatch. This
produced silent false-negatives in search result comparisons and lint checks
whenever a caller forgot to lowercase before comparing.

`Slug` already enforces structural invariants at construction (no `..`, no
leading `/`, no extension, no hidden components). Normalisation — lowercasing
all path segments — was the one remaining invariant enforced only by
convention.

## Design

`NormalizedSlug` is a newtype over `String`:

```rust
pub struct NormalizedSlug(String);
```

Construction paths:
- `Slug::normalize() -> NormalizedSlug` — public, the canonical path for
  external callers. Lowercases the validated slug string.
- `NormalizedSlug::from_normalized(String) -> NormalizedSlug` — `pub(crate)`,
  for internal index reads. Tantivy stores slugs already lowercased at index
  time; re-normalising on every read is redundant. The bypass is intentionally
  `pub(crate)` to prevent external callers from constructing unnormalized
  values.

`NormalizedSlug` implements:
- `Display`, `AsRef<str>`, `as_str()` — for formatting and string use
- `PartialEq<str>`, `PartialEq<&str>`, `PartialEq<String>` — so test
  assertions like `assert_eq!(result.slug, "concepts/moe")` compile unchanged
- `serde::Serialize/Deserialize` — serializes as a plain string (newtype
  default), so JSON output of `wiki_search` is unchanged

## Alternatives considered

**Normalise at `Slug::try_from` construction time.** Rejected — `Slug` is
used as a validated structural type throughout the engine (path resolution,
git operations, lint). Silently lowercasing at construction would change the
behaviour of `Slug::resolve()` on case-sensitive filesystems and make `Slug`
unsuitable as a round-trip path type.

**Add a `normalize()` method that returns `String`.** Rejected — returns a
plain `String`, so the compiler cannot distinguish a normalized slug from any
other string. The whole point is a distinct type.

**Normalise all slugs at index time and use `String` everywhere.** Already
done at index time. The problem is the comparison side: callers reading from
the index get a `String` and compare it to a `Slug`-derived value without
knowing whether either has been lowercased.

**Use a type alias `type NormalizedSlug = String`.** Rejected — type aliases
provide no compile-time distinction. `PartialEq` between `NormalizedSlug` and
`String` would be the same as `String == String`.

## Consequences

- `PageRef.slug` and `PageSummary.slug` are `NormalizedSlug` in the stable
  API surface. Callers use `.as_str()`, `.to_string()`, or direct `==`
  comparison with string literals (via `PartialEq<str>`).
- Internal construction sites (Tantivy field reads in `search.rs`) use
  `NormalizedSlug::from_normalized(slug_string)`.
- Test assertions in `tests/search.rs` compile unchanged due to the
  `PartialEq<str>` impl.
- JSON serialization of `wiki_search` results is unchanged — `NormalizedSlug`
  serializes as a plain string.
- `Slug` is unchanged. The two types are intentionally separate:
  `Slug` = structurally valid, `NormalizedSlug` = structurally valid +
  lowercased.
