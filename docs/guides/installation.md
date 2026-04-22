# Installation

## Quick Install

macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/geronimo-iia/llm-wiki/main/install.sh | bash
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/geronimo-iia/llm-wiki/main/install.ps1 | iex
```

The scripts detect your platform, download the latest binary from
GitHub releases, install it, and verify `git` is available.

Custom install directory:

```bash
LLM_WIKI_INSTALL_DIR=~/.local/bin curl -fsSL .../install.sh | bash
```

## From Source (cargo)

Requires [Rust](https://www.rust-lang.org/tools/install) 1.95+.

```bash
cargo install llm-wiki
```

## Pre-built Binary (cargo-binstall)

Requires [cargo-binstall](https://github.com/cargo-bins/cargo-binstall).

```bash
cargo binstall llm-wiki
```

Downloads a pre-built binary from GitHub releases — no compilation.

## Homebrew (macOS / Linux)

```bash
brew tap geronimo-iia/tap
brew install llm-wiki
```

## asdf Version Manager

```bash
asdf plugin add llm-wiki https://github.com/geronimo-iia/asdf-llm-wiki.git
asdf install llm-wiki latest
asdf global llm-wiki latest
```

## Manual Download·

Download a binary from the
[GitHub releases](https://github.com/geronimo-iia/llm-wiki/releases)
page. Available targets:

| Platform            | Archive                            |
| ------------------- | ---------------------------------- |
| Linux x86_64        | `x86_64-unknown-linux-gnu.tar.gz`  |
| Linux aarch64       | `aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel         | `x86_64-apple-darwin.tar.gz`       |
| macOS Apple Silicon | `aarch64-apple-darwin.tar.gz`      |
| Windows x86_64      | `x86_64-pc-windows-msvc.zip`       |

```bash
# Example: macOS Apple Silicon
curl -LO https://github.com/geronimo-iia/llm-wiki/releases/latest/download/aarch64-apple-darwin.tar.gz
tar xzf aarch64-apple-darwin.tar.gz
sudo mv llm-wiki /usr/local/bin/
```

## Verify

```bash
llm-wiki --version
```

## Prerequisites

- `git` — required for wiki repositories (commit, diff, history)
- No runtime dependencies — llm-wiki is a single static binary
