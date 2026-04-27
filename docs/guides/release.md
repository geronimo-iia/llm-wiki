# Release

## Distribution Channels

| Channel          | Command                                  | Notes               |
| ---------------- | ---------------------------------------- | ------------------- |
| Source build     | `cargo install llm-wiki-engine`          | Always available    |
| Pre-built binary | `cargo binstall llm-wiki`                | Via GitHub releases |
| Homebrew         | `brew install geronimo-iia/tap/llm-wiki` | macOS/Linux         |
| asdf             | `asdf install llm-wiki latest`           | Version manager     |

### Repositories

| Repo                                                           | Purpose            |
| -------------------------------------------------------------- | ------------------ |
| [llm-wiki](https://github.com/geronimo-iia/llm-wiki)           | Engine source + CI |
| [homebrew-tap](https://github.com/geronimo-iia/homebrew-tap)   | Homebrew formula   |
| [asdf-llm-wiki](https://github.com/geronimo-iia/asdf-llm-wiki) | asdf plugin        |

## Branch strategy

`main` is always releasable — tagged commits only. Feature work lands on a
`release/vX.Y.Z` integration branch, not directly on `main`.

```
feat/xxx  ─┐
feat/yyy  ─┼─▶  release/vX.Y.Z  ─▶  main  (tag vX.Y.Z)
feat/zzz  ─┘
```

1. Open `release/vX.Y.Z` from `main` at the start of the milestone.
2. Each `feat/...` PR targets `release/vX.Y.Z`, not `main`.
3. Run the pre-release checklist as commits on `release/vX.Y.Z`.
4. One final PR merges `release/vX.Y.Z` → `main`; tag on the merge commit.

Hotfixes branch from the relevant tag and merge back to both `main` and the
active `release/` branch if one is open.

## Milestone & tracking

- One GitHub milestone per release (`v0.2.0`, `v0.3.0`, …)
- One tracking issue per repo with a checklist of improvements
- One PR per improvement, linked to the tracking issue
- Improvement specs live in `docs/improvements/`; each has a `status` field
  (`proposed` → `implemented`) and a task checklist

## Pre-Release Checklist

### Code quality
- [ ] All tests pass: `cargo test`
- [ ] Formatted: `cargo fmt -- --check`
- [ ] No lint issues: `cargo clippy --all-targets -- -D warnings`
- [ ] No vulnerabilities: `cargo audit`

### Documentation pass
- [ ] All improvement spec files have `status: implemented` and tasks checked
- [ ] `docs/specifications/` reflects any changed tool signatures or config shapes
- [ ] `docs/guides/` covers every user-facing feature added in this release
- [ ] Public Rust types and functions have `///` rustdoc comments
- [ ] `CHANGELOG.md` section dated and complete

### Version
- [ ] Version bumped in `Cargo.toml`

## Release

```bash
# 1. Bump version in Cargo.toml, update CHANGELOG date

# 2. Commit on release branch
git commit -am "chore: release vx.y.z"

# 3. Merge release branch to main, then tag
git checkout main
git merge --no-ff release/vx.y.z
git tag -a vx.y.z -m "Release vx.y.z"
git push origin main
git push origin vx.y.z
```

Tagging triggers `.github/workflows/release.yml`:
1. Builds binaries for 5 targets (linux x86_64/aarch64, macOS x86_64/aarch64, windows x86_64)
2. Creates GitHub release with tarballs
3. Publishes to crates.io

Tags containing `-rc` (e.g. `v0.2.0-rc1`) follow the same build path but:
- The GitHub release is marked as **pre-release** (not shown as latest)
- The `publish` job is **skipped** — nothing is sent to crates.io

Install an RC binary directly:

```bash
cargo binstall llm-wiki@0.2.0-rc1   # reads GitHub releases
```

Or download the tarball manually from the GitHub releases page and put
`llm-wiki` on your `PATH`.

## Post-Release

### Homebrew formula

Update `homebrew-tap/Formula/llm-wiki.rb`:
- Version, URL, SHA256 for each platform
- Commit: `chore: bump llm-wiki to x.y.z`
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
