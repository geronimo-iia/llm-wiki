# Release

## Distribution Channels

| Channel | Command | Notes |
|---------|---------|-------|
| Source build | `cargo install llm-wiki` | Always available |
| Pre-built binary | `cargo binstall llm-wiki` | Via GitHub releases |
| Homebrew | `brew install geronimo-iia/tap/llm-wiki` | macOS/Linux |
| asdf | `asdf install llm-wiki latest` | Version manager |

### Repositories

| Repo | Purpose |
|------|---------|
| [llm-wiki](https://github.com/geronimo-iia/llm-wiki) | Engine source + CI |
| [homebrew-tap](https://github.com/geronimo-iia/homebrew-tap) | Homebrew formula |
| [asdf-llm-wiki](https://github.com/geronimo-iia/asdf-llm-wiki) | asdf plugin |

## Pre-Release Checklist

- [ ] All tests pass: `cargo test`
- [ ] Formatted: `cargo fmt -- --check`
- [ ] No lint issues: `cargo clippy -- -D warnings`
- [ ] No vulnerabilities: `cargo audit`
- [ ] `CHANGELOG.md` updated
- [ ] Version bumped in `Cargo.toml`

## Release

```bash
# 1. Bump version
# Edit Cargo.toml version field

# 2. Commit
git commit -am "chore: bump version to x.y.z"

# 3. Tag and push
git tag -a vx.y.z -m "Release vx.y.z"
git push origin main
git push origin vx.y.z
```

Tagging triggers `.github/workflows/release.yml`:
1. Builds binaries for 5 targets (linux x86_64/aarch64, macOS x86_64/aarch64, windows x86_64)
2. Creates GitHub release with tarballs
3. Publishes to crates.io

## Post-Release

### Homebrew formula

Update `homebrew-tap/Formula/llm-wiki.rb`:
- Version, URL, SHA256 for each platform
- Test: `brew install geronimo-iia/tap/llm-wiki`

### asdf plugin

Test: `asdf install llm-wiki <version>`

The plugin reads releases from GitHub — no update needed unless the
binary naming changes.

## Hotfix

```bash
git checkout -b hotfix/vx.y.z+1 vx.y.z
# Apply fix, bump patch version
git commit -am "fix: description"
git tag -a vx.y.z+1 -m "Hotfix vx.y.z+1"
git push origin hotfix/vx.y.z+1 vx.y.z+1
# Merge back to main
```
