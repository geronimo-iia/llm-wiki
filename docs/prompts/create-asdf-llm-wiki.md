# Create: asdf-llm-wiki Repository

## Objective

Create an asdf plugin at `geronimo-iia/asdf-llm-wiki`. Users install
with:

```bash
asdf plugin add llm-wiki https://github.com/geronimo-iia/asdf-llm-wiki.git
asdf install llm-wiki latest
asdf global llm-wiki latest
```

## Based on

`/Users/geronimo/build/sp_theory/projects/asdf-agentctl` — existing
plugin. Same structure, replace `agentctl` with `llm-wiki`.

## Create

Repository: `geronimo-iia/asdf-llm-wiki`

```
asdf-llm-wiki/
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── bin/
│   ├── download
│   ├── install
│   └── list-all
├── lib/
│   └── utils.bash
└── test.sh
```

### lib/utils.bash

Three changes from the agentctl version:

```bash
GH_REPO="https://github.com/geronimo-iia/llm-wiki"
TOOL_NAME="llm-wiki"
TOOL_TEST="llm-wiki --version"
```

Everything else (platform detection, download, install) is identical —
the release workflow produces the same tarball structure
(`<target>.tar.gz` containing the binary at the root).

### bin/download, bin/install, bin/list-all

Copy from `asdf-agentctl/bin/` unchanged — they source `utils.bash`
and use `$TOOL_NAME` throughout.

### README.md

```markdown
# asdf-llm-wiki

[llm-wiki](https://github.com/geronimo-iia/llm-wiki) plugin for
[asdf](https://asdf-vm.com/) version manager.

## Install

    asdf plugin add llm-wiki https://github.com/geronimo-iia/asdf-llm-wiki.git
    asdf install llm-wiki latest
    asdf global llm-wiki latest

## Use

    llm-wiki --version
```

### test.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

asdf plugin add llm-wiki .
asdf install llm-wiki latest
asdf global llm-wiki latest
llm-wiki --version
```

## Notes

- The plugin reads GitHub tags to list versions — no update needed
  when new versions are released
- Platform detection supports: macOS arm64/x86_64, Linux x86_64
- Linux aarch64 support: add to the `case` in `utils.bash` if needed
  (the release workflow builds it)
