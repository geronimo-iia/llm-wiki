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

## Branch Strategy

`main` is always releasable — tagged commits only. Feature work lands on a
`release/vX.Y.Z` integration branch, not directly on `main`.

```
feat/xxx  ─┐
feat/yyy  ─┼─▶  release/vX.Y.Z  ─▶  main  (tag vX.Y.Z)
feat/zzz  ─┘
```

1. Complete the [Pre-Release Checklist](#pre-release-checklist) on `main` before branching.
2. Open `release/vX.Y.Z` from `main` at the start of the milestone.
3. Each `feat/...` PR targets `release/vX.Y.Z`, not `main`.
4. Run the pre-release checklist as commits on `release/vX.Y.Z`.
5. One final PR merges `release/vX.Y.Z` → `main`; tag on the merge commit.

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
- [ ] Doc tests pass: `cargo test --doc`
- [ ] Formatted: `cargo fmt --check`
- [ ] No lint issues: `cargo clippy --all-targets -- -D warnings`
- [ ] Release build clean: `cargo build --release --locked`
- [ ] Integration tests green: trigger **Integration Tests** workflow (`suite: both`) on the release branch

### Documentation

- [ ] All improvement spec files have `status: implemented` and tasks checked
- [ ] `docs/specifications/` reflects any changed tool signatures or config shapes
- [ ] `docs/guides/` covers every user-facing feature added in this release
- [ ] Public Rust types and functions have `///` rustdoc comments; `cargo doc --no-deps` zero warnings
- [ ] `CHANGELOG.md` section dated and complete

### Version

- [ ] Version bumped in `Cargo.toml`
- [ ] `Cargo.lock` updated: `cargo update -p llm-wiki-engine`

## Release

```bash
# 1. Bump version in Cargo.toml, run cargo update -p llm-wiki-engine, update CHANGELOG date

# 2. Commit on release branch, push, open PR
git commit -am "chore: release vx.y.z"
git push origin release/vx.y.z
gh pr create --title "chore: release vx.y.z" --base main

# 3. Wait for PR CI to pass, then merge to main
git checkout main
git merge --no-ff release/vx.y.z

# 4. Tag the merge commit and push — GitHub Actions handles the rest
git tag -a vx.y.z -m "Release vx.y.z"
git push origin main
git push origin vx.y.z
# GitHub Actions: builds 5 targets, creates GitHub release, publishes to crates.io
```

Tags containing `-rc` (e.g. `v0.3.0-rc1`) follow the same steps but:
- GitHub release is marked **pre-release** (not shown as latest)
- `publish` job is **skipped** — nothing sent to crates.io

Install an RC binary directly:

```bash
cargo binstall llm-wiki@0.3.0-rc1   # reads GitHub releases
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
# Apply fix, bump patch version in Cargo.toml
git commit -am "fix: description"
git tag -a vx.y.z+1 -m "Hotfix vx.y.z+1"
git push origin hotfix/vx.y.z+1 vx.y.z+1
# Merge back to main
git checkout main
git merge --no-ff hotfix/vx.y.z+1
git push origin main
```

## CHANGELOG Format

Move `[Unreleased]` entries to a versioned section:

```markdown
## [0.3.0] — 2026-MM-DD

### Added
- …

### Fixed
- …

## [Unreleased]
```
