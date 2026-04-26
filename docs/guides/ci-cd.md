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
    steps:
      - uses: actions/checkout@v4

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
    steps:
      - uses: actions/checkout@v4

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
    steps:
      - uses: actions/checkout@v4

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

## Environment Notes

- llm-wiki creates state at `~/.llm-wiki/` — this is ephemeral in CI
- `spaces create` is idempotent — safe to run on every build
- `--dry-run` on ingest validates without committing
- The wiki repo must be a git repository (`checkout@v4` handles this)
