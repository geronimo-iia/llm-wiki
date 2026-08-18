# `redact_error` does not redact Windows paths

## Decision

Ship 1.0.0 with `redact_error` covering Unix absolute paths and tilde-prefixed
paths only. Windows drive-letter paths (`C:\…`) and UNC paths (`\\server\share`)
are not redacted. Revisit when a Windows contributor can test the fix end-to-end.

## Context

`redact_error` in `src/mcp/handlers.rs` strips filesystem paths from error
strings before returning them to LLM clients, preventing workspace layout leakage.
The current regex covers two forms:

```
/[a-zA-Z0-9_./-]{3,}    — Unix absolute paths starting with /
~[a-zA-Z0-9_./~-]{2,}   — tilde-prefixed paths (~/… or ~user/…)
```

Windows paths take two additional forms not matched by this regex:

- **Drive-letter paths:** `C:\Users\alice\wikis\my-wiki\state.toml`
- **UNC paths:** `\\server\share\wikis\my-wiki` or (after `canonicalize`)
  `\\?\UNC\server\share\wikis\my-wiki`

A Windows build of `llm-wiki` would emit these forms in anyhow error chains.
They would pass through `redact_error` unmodified, leaking the filesystem layout
to LLM clients.

## Why deferred

- No maintainer with a Windows environment can run `cargo test` and verify the
  fix does not over-redact (e.g. `C:` alone should not match; short UNC prefixes
  should not match).
- The existing Windows-specific code (`validate_wiki_root`, verbatim-prefix
  stripping) was contributed by a Windows user and tested in CI. The same bar
  applies here.
- The primary deployment target is Linux/macOS. Windows support is best-effort.

## What the fix looks like

Add two alternatives to the `PATH_RE` regex in `src/mcp/handlers.rs`:

```rust
regex::Regex::new(
    r"(?:/[a-zA-Z0-9_./-]{3,}|~[a-zA-Z0-9_./~-]{2,}|[A-Za-z]:\\[^\s]{3,}|\\\\[^\s]{3,})"
).unwrap()
```

- `[A-Za-z]:\\[^\s]{3,}` — drive-letter paths (`C:\…`), requiring at least 3
  non-whitespace chars after the backslash to avoid false positives on short
  strings like `C:\x`.
- `\\\\[^\s]{3,}` — UNC paths (`\\server\…`), same length guard.

The fix must be accompanied by unit tests using the `redact_error` helper
covering: drive-letter path in error string, UNC path in error string, short
strings that must not be redacted, and mixed Unix+Windows paths in one message.

## Consequences

- Windows users of `llm-wiki serve` may see local filesystem paths in MCP error
  responses. This is an information leak but not a security boundary violation —
  the MCP client is a trusted local tool in the expected deployment model.
- The gap is documented here so a Windows contributor can find it, implement the
  fix, and add the required tests without re-investigating the background.
