# Create: homebrew-tap Repository

## Objective

Create a Homebrew tap repository at `geronimo-iia/homebrew-tap` with
a formula for llm-wiki. Users install with:

```bash
brew tap geronimo-iia/tap
brew install llm-wiki
```

## Based on

`/Users/geronimo/build/sp_theory/projects/homebrew-agent` — existing
tap with `agentctl.rb`. Use the same structure.

## Create

Repository: `geronimo-iia/homebrew-tap`

```
homebrew-tap/
├── README.md
├── Formula/
│   └── llm-wiki.rb
```

### Formula/llm-wiki.rb

```ruby
class LlmWiki < Formula
  desc "Git-backed wiki engine with MCP server"
  homepage "https://github.com/geronimo-iia/llm-wiki"
  version "0.1.0"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/geronimo-iia/llm-wiki/releases/download/v0.1.0/aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/geronimo-iia/llm-wiki/releases/download/v0.1.0/x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/geronimo-iia/llm-wiki/releases/download/v0.1.0/aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/geronimo-iia/llm-wiki/releases/download/v0.1.0/x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "llm-wiki"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/llm-wiki --version")
  end
end
```

### README.md

```markdown
# homebrew-tap

Homebrew formulae for geronimo-iia tools.

## Install

    brew tap geronimo-iia/tap
    brew install llm-wiki

## Formulae

| Formula | Description |
|---------|-------------|
| llm-wiki | Git-backed wiki engine with MCP server |
```

## Update Process

After each llm-wiki release:
1. Download the release tarballs
2. Compute SHA256: `shasum -a 256 *.tar.gz`
3. Update version, URLs, and SHA256 in `Formula/llm-wiki.rb`
4. Commit and push

## Notes

- SHA256 placeholders must be replaced after the first release
- Linux aarch64 support added (the release workflow builds it via cross)
- The old `homebrew-agent` repo can be archived once `agentctl.rb`
  is migrated here (or kept separate — your choice)
