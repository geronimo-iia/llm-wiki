# Improvements


## ACP streaming

→ [acp-tasks.md](acp-tasks.md) · [ACP SDK reference](implementation/acp-sdk.md) · [ACP transport spec](specifications/integrations/acp-transport.md)


## ingest

→ [ingest-auto-commit.md](ingest-auto-commit.md)


## monitoring

- log → [logging.md](logging.md) · [logging-tasks.md](logging-tasks.md)
- server thread auto restart → [server-resilience.md](server-resilience.md)
- index corruption detection / rebuild → [index-corruption.md](index-corruption.md)


## release

- [ ] Fix `docs/release.md` — currently a verbatim copy of agentctl's; still says "agentctl" in title and post-release sections
- [ ] Replace agentctl-specific post-release steps (homebrew formula `agentctl.rb`, `agent-skills/agentctl`, `asdf-agentctl`, `chocolatey-agentctl`) with llm-wiki equivalents or remove if not applicable yet
- [ ] Decide which distribution channels apply to llm-wiki (homebrew, asdf, chocolatey, cargo-binstall) and document only those

## target & distribution

Reference: agentctl already has all of the below working — align llm-wiki.

### CI workflow (`ci.yml`)
- [ ] Upgrade actions to v6 (`actions/checkout@v6`)
- [ ] Add `feat/**` branch trigger (agentctl has it, llm-wiki only triggers on `main`)
- [ ] Use `cargo test --locked` instead of `cargo test --verbose`
- [ ] Use `rustsec/audit-check@v2` action instead of manual `cargo install cargo-audit`

### Release workflow (`release.yml`)
- [ ] Add `aarch64-unknown-linux-gnu` target via `cross` (agentctl has it, llm-wiki doesn't)
- [ ] Add `x86_64-pc-windows-msvc` target (agentctl has it, llm-wiki doesn't)
- [ ] Upgrade actions to v6/v7/v8 (`checkout@v6`, `upload-artifact@v7`, `download-artifact@v8`, `action-gh-release@v2`)
- [ ] Add `--locked` flag to release build
- [ ] Package Windows build as `.zip` instead of `.tar.gz`
- [ ] Pass `CARGO_TOKEN` via `--token` flag instead of env var (match agentctl style)

### Cargo.toml
- [ ] Add `rust-version` field (agentctl has `"1.75"`)
- [ ] Add `homepage`, `documentation`, `keywords`, `categories` fields
- [ ] Add `exclude` for `.github/`, `tests/`, `docs/`, `tests-beta/`, `src-beta/`
- [ ] Add `panic = "abort"` to `[profile.release]` (agentctl has it)

### Distribution channels (new repos/configs needed)
- [ ] Homebrew formula in `homebrew-agent` tap (`Formula/llm-wiki.rb` or `Formula/wiki.rb`)
- [ ] asdf plugin (`asdf-llm-wiki`)
- [ ] Chocolatey package (`chocolatey-llm-wiki`)
- [ ] Verify `cargo-binstall` metadata works (already has `[package.metadata.binstall]`)
