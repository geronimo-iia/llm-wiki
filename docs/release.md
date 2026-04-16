# Release Checklist

Complete release process for agentctl following [agent-software standards](https://github.com/geronimo-iia/agent-foundation/tree/main/agent-software).

## Pre-Release

### Code Quality
- [ ] All tests pass: `cargo test`
- [ ] Code formatted: `cargo fmt`
- [ ] No linting issues: `cargo clippy -- -D warnings`
- [ ] No security vulnerabilities: `cargo audit`
- [ ] All phase exit criteria completed

### Documentation
- [ ] Update `CHANGELOG.md` per [changelog format](https://github.com/geronimo-iia/agent-foundation/blob/main/agent-software/version-control-release/changelog-format.md)
- [ ] Update `README.md` if needed
- [ ] Document new features in relevant docs/ files
- [ ] Update version in `Cargo.toml`

### Testing
- [ ] Integration tests pass against real repositories
- [ ] Manual testing completed
- [ ] Backward compatibility verified

## Release

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md` with new version entry
- [ ] Run final quality checks (`fmt`, `clippy`, `audit`, `test`)
- [ ] Build release binary: `cargo build --release`
- [ ] Commit with [semantic message](https://github.com/geronimo-iia/agent-foundation/blob/main/agent-software/version-control-release/git-commit-semantic.md)
- [ ] Create annotated tag: `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
- [ ] Push commits: `git push origin main`
- [ ] Push tag: `git push origin vX.Y.Z`
- [ ] Verify GitHub Actions CI passes
- [ ] Verify release binaries built
- [ ] Verify GitHub release created

## Post-Release

### Homebrew Formula
- [ ] Navigate to `homebrew-agent` repository
- [ ] Update `Formula/agentctl.rb` (version, URL, SHA256)
- [ ] Update homebrew repo `CHANGELOG.md`
- [ ] Update homebrew repo `README.md` if needed
- [ ] Commit and push homebrew changes
- [ ] Test: `brew install geronimo-iia/agent/agentctl`

### Agent Skills Update
- [ ] Navigate to `agent-skills/agentctl` directory
- [ ] Update skill to reference new agentctl version
- [ ] Update skill documentation if needed
- [ ] Update agent-skills `CHANGELOG.md`
- [ ] Commit and push skill changes

### ASDF Plugin Update
- [ ] Navigate to `asdf-agentctl` repository
- [ ] Test plugin works with new release: `./test.sh`
- [ ] Update `CHANGELOG.md` if plugin changes needed
- [ ] Commit and push any updates
- [ ] Test: `asdf install agentctl latest`

### Chocolatey Package Update
- [ ] Navigate to `chocolatey-agentctl` repository
- [ ] Verify GitHub Actions workflow triggered automatically
- [ ] Monitor workflow completion and package creation
- [ ] Verify package published to Chocolatey community
- [ ] Test: `choco install agentctl` or `choco upgrade agentctl`

### Communication
- [ ] Update external documentation
- [ ] Update installation instructions
- [ ] Announce release
- [ ] Update dependent projects
- [ ] Monitor for issues
- [ ] Monitor CI/CD pipelines

## Hotfix Process

- [ ] Create hotfix branch: `git checkout -b hotfix/vX.Y.Z+1 vX.Y.Z`
- [ ] Apply minimal fix
- [ ] Update `CHANGELOG.md` with patch entry
- [ ] Bump patch version in `Cargo.toml`
- [ ] Test: `cargo test`
- [ ] Commit: `git commit -m "fix: description"`
- [ ] Tag: `git tag -a vX.Y.Z+1 -m "Hotfix vX.Y.Z+1"`
- [ ] Push branch and tag
- [ ] Merge back to main

## Rollback Process

### Immediate Response
- [ ] Document issue and impact
- [ ] Communicate via GitHub issue

### Quick Fix
- [ ] Follow hotfix process if simple

### Revert Release
- [ ] Revert problematic commit if complex
- [ ] Create new patch release

## Standards

- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Semantic Commits](https://github.com/geronimo-iia/agent-foundation/blob/main/agent-software/version-control-release/git-commit-semantic.md)
- [Agent Software Standards](https://github.com/geronimo-iia/agent-foundation/tree/main/agent-software)