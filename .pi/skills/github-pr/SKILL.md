---
name: github-pr
description: GitHub pull request workflow for Chromancy. Provides PR creation checklists, code review prompts, branch naming conventions, and release preparation steps. Use when creating PRs, reviewing code, or preparing releases on the main branch.
---

# GitHub PR Workflow for Chromancy

## Branch Naming

- `feature/<name>` — New features
- `fix/<name>` — Bug fixes
- `refactor/<name>` — Code refactoring
- `docs/<name>` — Documentation changes
- `release/<version>` — Release preparation

## PR Creation Checklist

Before opening a PR, verify:

```bash
# 1. Branch is up to date with main
git fetch origin
git rebase origin/main

# 2. Tests pass
cargo test

# 3. Code compiles without warnings
cargo check
cargo clippy -- -D warnings

# 4. Formatting is clean
cargo fmt --check

# 5. No debug prints or leftover TODOs
grep -r "println!\|dbg!\|TODO\|FIXME" src/ || true
```

## PR Template

When creating a PR, include:

```markdown
## Summary
Brief description of changes

## Type
- [ ] Feature
- [ ] Bug fix
- [ ] Refactor
- [ ] Documentation
- [ ] Release prep

## Checklist
- [ ] Tests pass (`cargo test`)
- [ ] Clippy clean (`cargo clippy`)
- [ ] Formatted (`cargo fmt`)
- [ ] No debug prints
- [ ] Rebased on main
- [ ] CHANGELOG.md updated (if applicable)

## Testing
How was this tested?

## Breaking Changes
Any API changes?
```

## Review Prompt

When reviewing a PR, focus on:

1. **Correctness** — Does the code do what it claims?
2. **Error handling** — Are all `Result` paths handled? Are errors contextual per `WledError`?
3. **Async safety** — Are there any blocking operations in async contexts?
4. **API alignment** — Does it mirror WLED's JSON API closely?
5. **Test coverage** — Are there unit tests with mocked responses?
6. **Documentation** — Are public APIs documented?
7. **Multi-device concerns** — Does it handle fleet/sync group edge cases?

## Release Preparation

For release PRs:

1. Update `CHANGELOG.md`
2. Bump version in `Cargo.toml`
3. Verify all `TODO`s in `TASKS.md` are resolved or moved
4. Run full test suite
5. Update README if API changed
6. Tag after merge: `git tag vX.Y.Z`

## Commands

```bash
# Quick status check
./scripts/pr-check.sh

# Full validation
./scripts/pr-check.sh --full
```
