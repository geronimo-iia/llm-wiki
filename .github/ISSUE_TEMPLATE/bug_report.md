---
name: Bug report
about: Report incorrect behaviour in the wiki engine
labels: bug
---

## Environment

- `llm-wiki --version` output:
- OS and architecture:
- Install method (cargo, binstall, homebrew, asdf, script):

## Command run

```bash
# paste the exact command (sanitise any sensitive paths or content)
llm-wiki ingest wiki/concepts/example.md
```

## Expected behaviour

<!-- What should have happened? -->

## Actual behaviour

<!-- What happened instead? -->

## Debug log

```
# RUST_LOG=llm_wiki=debug llm-wiki <your command>
```

## Minimal reproduction

<!-- If possible, provide a minimal wiki setup that reproduces the issue:
     - wiki.toml content
     - schema file (if custom type)
     - page frontmatter
-->
