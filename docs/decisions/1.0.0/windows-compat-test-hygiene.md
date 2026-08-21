# Windows compatibility: test and integration hygiene

## Decision

Apply six cross-platform hygiene rules to all Rust unit tests and Python
integration tests to eliminate Windows CI failures that do not reproduce on
Linux or macOS.

## Context

The `integration-windows` CI job runs `cargo test` and `pytest engine/` on
`windows-latest`. A sequence of failures during the 1.0.0-rc2 cycle revealed
that several test assumptions are Unix-specific. Each rule below corresponds to
a class of failure observed in CI.

## Rules

### 1. Gate Unix-only code with `#[cfg(unix)]`

`std::os::unix::fs::PermissionsExt` and `std::os::unix::fs::symlink` do not
exist on Windows. Any test function that uses them must be annotated
`#[cfg(unix)]`. Variables declared outside a `#[cfg(unix)]` block but used only
inside one must be prefixed with `_` to suppress the `unused_variable` warning
that `cargo test` emits on Windows (where the block is compiled out).

### 2. Use `USERPROFILE` as fallback for `HOME`

The `HOME` environment variable is set on Unix but absent on Windows, where the
equivalent is `USERPROFILE`. Production code that resolves a default log or
data path must try `HOME` first, then `USERPROFILE`, then fall back to `"."`:

```rust
std::env::var("HOME")
    .or_else(|_| std::env::var("USERPROFILE"))
    .unwrap_or_else(|_| ".".into())
```

### 3. Use `Path::ends_with` for path suffix checks

`String::ends_with(".llm-wiki/logs")` fails on Windows because path separators
are backslashes. Use `std::path::Path::ends_with(std::path::Path::new("..."))`,
which compares path components regardless of separator:

```rust
assert!(Path::new(&cfg.log_path).ends_with(Path::new(".llm-wiki/logs")));
```

### 4. Canonicalize both sides when comparing absolute paths

`Path::canonicalize()` on macOS resolves `/var/…` to `/private/var/…` (symlink
in the OS). On Windows it prepends `\\?\` (verbatim UNC prefix). When asserting
that two path values refer to the same location, canonicalize both sides so
platform-specific transformations cancel:

```rust
assert_eq!(space.wiki_root.canonicalize()?, wiki_path.canonicalize()?.join("wiki"));
```

The engine already strips `\\?\` from user-facing values via `strip_verbatim_prefix`
in `src/pathutil.rs`. Tests compare raw stored values, so they need the
canonicalize-both-sides pattern.

### 5. Force UTF-8 encoding for subprocess output and file I/O

Python's `subprocess.run(..., text=True)` and `Path.read_text()` / `.write_text()`
default to the system locale encoding — `cp1252` on Windows. Wiki fixture files
contain UTF-8 content (em-dashes, curly quotes, Unicode in page body text). The
Rust binary always writes UTF-8. All calls must declare the encoding explicitly:

```python
# subprocess
subprocess.run([...], capture_output=True, text=True, encoding="utf-8")

# file I/O
path.read_text(encoding="utf-8")
path.write_text(content, encoding="utf-8")
```

### 6. No hardcoded Unix paths in pytest configuration

`pyproject.toml` `addopts` must not reference Unix-only paths. The option
`--basetemp=/tmp/llm-wiki-tests` causes every pytest `tmp_path` fixture to
fail on Windows because `D:\tmp` does not exist. Remove `--basetemp`; pytest
uses a per-OS default temp directory automatically.

## Why these are rules, not one-off fixes

Each class of failure has a common root — an implicit Unix assumption — and each
is easy to re-introduce by any contributor working on macOS or Linux. Documenting
the rules here makes them findable during code review without requiring a live
Windows environment.

## Consequences

- `tests/spaces.rs`: test functions using `PermissionsExt` or `symlink` are
  annotated `#[cfg(unix)]`; variables used only inside such blocks are
  declared with a leading `_`.
- `src/config.rs`: `default_log_path()` tries `USERPROFILE` after `HOME`.
- `tests/config.rs`: path assertions use `Path::ends_with`.
- `tests/engine.rs`: wiki-root assertions canonicalize both sides.
- `tests-integration/conftest.py`: both `subprocess.run` calls include
  `encoding="utf-8"`.
- `tests-integration/engine/test_export.py`, `test_incremental.py`,
  `test_ingest.py`, `test_spaces.py`: file I/O calls include
  `encoding="utf-8"`.
- `tests-integration/pyproject.toml`: `--basetemp` removed from `addopts`.
