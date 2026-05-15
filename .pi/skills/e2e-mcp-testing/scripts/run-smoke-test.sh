#!/usr/bin/env bash
# Run the full MCP read-only smoke test against live WLED hardware.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"

BINARY="${REPO_ROOT}/target/debug/chromancy"
CONFIG="${REPO_ROOT}/wled-config.toml"
TEST_SCRIPT="${REPO_ROOT}/tests/e2e/mcp-readonly-smoke-test.mjs"
VALIDATE_SCRIPT="${SCRIPT_DIR}/validate-config.sh"

die() { echo "[e2e-smoke] ERROR: $*" >&2; exit 1; }

[[ -f "$BINARY" ]] || die "Binary not found: $BINARY — run 'cargo build' first"
[[ -f "$TEST_SCRIPT" ]] || die "Test script not found: $TEST_SCRIPT"
command -v node >/dev/null 2>&1 || die "node is required but not installed"

# Validate/preview the config and prompt user for confirmation.
"$VALIDATE_SCRIPT"

echo "[e2e-smoke] Starting MCP smoke test against live fleet..."
echo "[e2e-smoke] Binary: $BINARY"
echo "[e2e-smoke] Config:  $CONFIG"
echo ""

cd "$REPO_ROOT"
node "$TEST_SCRIPT"

echo ""
echo "[e2e-smoke] Smoke test complete."
