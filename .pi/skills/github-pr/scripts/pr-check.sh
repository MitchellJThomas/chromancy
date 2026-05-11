#!/bin/bash
# PR validation script for Chromancy

set -e

FULL=0
if [ "$1" == "--full" ]; then
    FULL=1
fi

echo "=== Chromancy PR Check ==="

# Check we're on a feature branch, not main
BRANCH=$(git branch --show-current)
if [ "$BRANCH" == "main" ]; then
    echo "❌ ERROR: You are on main branch. Create a feature branch first."
    exit 1
fi

echo "Branch: $BRANCH"

# Check branch naming
if [[ ! "$BRANCH" =~ ^(feature|fix|refactor|docs|release)/ ]]; then
    echo "⚠️  WARNING: Branch name doesn't follow convention (feature/fix/refactor/docs/release/name)"
fi

# Check for uncommitted changes
if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "⚠️  WARNING: You have uncommitted changes"
fi

# Fetch and check if behind main
git fetch origin main --quiet
BEHIND=$(git rev-list --count HEAD..origin/main)
if [ "$BEHIND" -gt 0 ]; then
    echo "⚠️  WARNING: Branch is $BEHIND commits behind main. Rebase recommended."
fi

echo ""
echo "=== Rust Checks ==="

# Format check
echo "Checking formatting..."
if ! cargo fmt --check; then
    echo "❌ Formatting issues found. Run: cargo fmt"
    exit 1
fi
echo "✓ Formatting OK"

# Compilation
echo "Checking compilation..."
cargo check --quiet
echo "✓ Compiles"

# Clippy (only in full mode)
if [ "$FULL" -eq 1 ]; then
    echo "Running clippy..."
    cargo clippy -- -D warnings
    echo "✓ Clippy clean"
fi

# Tests
echo "Running tests..."
cargo test --quiet
echo "✓ Tests pass"

# Check for debug prints
echo "Checking for debug prints..."
if grep -r "println!\|dbg!" src/ 2>/dev/null; then
    echo "⚠️  WARNING: Found println! or dbg! macros"
fi

# Check for TODO/FIXME in new code
echo "Checking for TODOs/FIXMEs..."
if grep -r "TODO\|FIXME" src/ 2>/dev/null; then
    echo "ℹ️  Note: Found TODO/FIXME comments (review before merge)"
fi

echo ""
echo "=== PR Check Complete ==="
echo "✓ Ready to create PR"
