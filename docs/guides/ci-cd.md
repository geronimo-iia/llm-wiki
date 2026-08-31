# CI/CD Integration

llm-wiki is a single binary with no runtime dependencies. It runs in
any CI environment that has `git`.

## Install in CI

```yaml
# GitHub Actions
- name: Install llm-wiki
  run: cargo binstall llm-wiki --no-confirm

# Or from source (slower, no cargo-binstall needed)
- name: Install llm-wiki
  run: cargo install llm-wiki-engine --locked
```

## Schema Validation on PR

Validate that all pages pass frontmatter validation and all schemas
are well-formed. Fails the build if a page has invalid frontmatter
(in strict mode) or a schema file is broken.

```yaml
name: Wiki Lint

on:
  pull_request:
    paths:
      - 'wiki/**'
      - 'schemas/**'
      - 'wiki.toml'

jobs:
  validate:
    runs-on: ubuntu-latest
    env:
      LLM_WIKI_CONFIG: ${{ runner.temp }}/llm-wiki.toml
    steps:
      - uses: actions/checkout@v6

      - name: Install llm-wiki
        run: cargo binstall llm-wiki --no-confirm

      - name: Register wiki
        run: llm-wiki spaces create . --name ci

      - name: Validate schemas
        run: llm-wiki schema validate --wiki ci

      - name: Ingest (dry run)
        run: llm-wiki ingest wiki/ --dry-run --wiki ci
```

## Index Rebuild on Merge

Rebuild the search index after content changes land on main. Useful
if the index is stored as a CI artifact or deployed alongside a
static site.

```yaml
name: Rebuild Index

on:
  push:
    branches: [main]
    paths:
      - 'wiki/**'
      - 'schemas/**'

jobs:
  rebuild:
    runs-on: ubuntu-latest
    env:
      LLM_WIKI_CONFIG: ${{ runner.temp }}/llm-wiki.toml
    steps:
      - uses: actions/checkout@v6

      - name: Install llm-wiki
        run: cargo binstall llm-wiki --no-confirm

      - name: Register wiki
        run: llm-wiki spaces create . --name ci

      - name: Rebuild index
        run: llm-wiki index rebuild --wiki ci

      - name: Index status
        run: llm-wiki index status --wiki ci --format json
```

## Ingest on PR Merge

Automatically validate and commit after content is merged. Useful
for wikis where an LLM writes pages via PR and the engine validates
on merge.

```yaml
name: Auto Ingest

on:
  push:
    branches: [main]
    paths:
      - 'wiki/**'

jobs:
  ingest:
    runs-on: ubuntu-latest
    env:
      LLM_WIKI_CONFIG: ${{ runner.temp }}/llm-wiki.toml
    steps:
      - uses: actions/checkout@v6

      - name: Install llm-wiki
        run: cargo binstall llm-wiki --no-confirm

      - name: Register wiki
        run: llm-wiki spaces create . --name ci

      - name: Ingest all
        run: llm-wiki ingest wiki/ --wiki ci
```

## Pre-commit Hook

Validate frontmatter locally before committing. Add to
`.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: wiki-validate
        name: Validate wiki pages
        entry: bash -c 'llm-wiki spaces create . --name local 2>/dev/null; llm-wiki ingest wiki/ --dry-run --wiki local'
        language: system
        files: '^wiki/.*\.md$'
        pass_filenames: false
```

Or as a git hook in `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e
llm-wiki spaces create . --name local 2>/dev/null || true
llm-wiki ingest wiki/ --dry-run --wiki local
```

## Graph Generation in CI

Generate a concept graph as a build artifact or commit it to the repo:

```yaml
      - name: Generate graph
        run: |
          llm-wiki graph --format mermaid --output wiki/graph.md --wiki ci
          llm-wiki graph --format dot --output wiki/graph.dot --wiki ci
```

## Integration Test Workflow

The integration suite lives in `tests-integration/` — a pytest project managed
by `uv`. Three suites cover all transports:

| Suite | Transport | Target |
|---|---|---|
| `engine/` | CLI subprocess | `make validate-py-engine` |
| `mcp/` | MCP stdio (official `mcp` SDK) | `make validate-py-mcp` |
| `acp/` | ACP NDJSON stdio (`asyncio`) | `make validate-py-acp` |

Run all suites locally:

```bash
make validate-py
```

Run a specific suite:

```bash
make validate-py-engine
make validate-py-mcp
make validate-py-acp
```

The GitHub Actions workflow at `.github/workflows/integration.yml` triggers
automatically on pushes to `main` that touch `src/**` or `tests-integration/**`,
and can also be run manually from the Actions tab → **Integration Tests** →
**Run workflow** with a `suite` input (`all`, `engine`, `mcp`, `acp`).

The workflow:
1. Builds the debug binary (`cargo build --locked`)
2. Installs Python deps via `uv sync`
3. Runs the selected pytest suite(s)

No external tools required (`jq`, `mcptools`, etc.). Dependencies are declared
in `tests-integration/pyproject.toml`.

Use this after merging features that touch MCP handlers, ACP workflows, graph
rendering, or ingest logic — areas not covered by unit tests alone.

## Cross-platform Test Hygiene

The integration CI job runs `cargo test` and `pytest engine/` on `windows-latest`.
Six rules eliminate Windows CI failures that do not reproduce on Linux or macOS.
Apply all six to any new test or fixture.

### 1. Gate Unix-only code with `#[cfg(unix)]`

`std::os::unix::fs::PermissionsExt` and `std::os::unix::fs::symlink` do not exist
on Windows. Annotate any test function that uses them with `#[cfg(unix)]`.
Variables declared outside such a block but used only inside it must be prefixed
with `_` to suppress the `unused_variable` warning emitted on Windows.

### 2. Use `USERPROFILE` as fallback for `HOME`

`HOME` is absent on Windows; the equivalent is `USERPROFILE`. Production code that
resolves a default path must try `HOME` first, then `USERPROFILE`, then fall back:

```rust
std::env::var("HOME")
    .or_else(|_| std::env::var("USERPROFILE"))
    .unwrap_or_else(|_| ".".into())
```

### 3. Use `Path::ends_with` for path suffix checks

`String::ends_with(".llm-wiki/logs")` fails on Windows because path separators are
backslashes. Use `std::path::Path::ends_with` instead — it compares path components
regardless of separator:

```rust
assert!(Path::new(&cfg.log_path).ends_with(Path::new(".llm-wiki/logs")));
```

### 4. Canonicalize both sides when comparing absolute paths

`Path::canonicalize()` on macOS resolves `/var/…` to `/private/var/…` and on
Windows prepends `\\?\`. When asserting that two paths refer to the same location,
canonicalize both sides:

```rust
assert_eq!(space.wiki_root.canonicalize()?, wiki_path.canonicalize()?.join("wiki"));
```

The engine strips `\\?\` from user-facing values via `strip_verbatim_prefix` in
`src/pathutil.rs`. Tests compare raw stored values, so they need this pattern.

### 5. Force `encoding="utf-8"` on subprocess output and file I/O

Python's `subprocess.run(..., text=True)` and `Path.read_text()` / `.write_text()`
default to the system locale encoding (`cp1252` on Windows). Wiki fixture files
contain UTF-8 content. Always declare the encoding explicitly:

```python
# subprocess
subprocess.run([...], capture_output=True, text=True, encoding="utf-8")

# file I/O
path.read_text(encoding="utf-8")
path.write_text(content, encoding="utf-8")
```

### 6. No hardcoded Unix paths in pytest configuration

`pyproject.toml` `addopts` must not reference Unix-only paths. The option
`--basetemp=/tmp/llm-wiki-tests` causes every pytest `tmp_path` fixture to fail on
Windows because `/tmp` does not exist. Remove `--basetemp`; pytest selects a
per-OS default temp directory automatically.

## Environment Notes

- llm-wiki writes its space registry to `~/.llm-wiki/config.toml` by default
- In CI, set `LLM_WIKI_CONFIG` to a temp path to avoid touching `~/.llm-wiki/`:
  ```yaml
  env:
    LLM_WIKI_CONFIG: ${{ runner.temp }}/llm-wiki.toml
  ```
  Or pass `--config` to individual commands when env vars are not practical.
- `spaces create` is idempotent — safe to run on every build
- `--dry-run` on ingest validates without committing
- The wiki repo must be a git repository (`actions/checkout` handles this)
