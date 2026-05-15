#!/usr/bin/env bash
# Validate and preview wled-config.toml before running live hardware tests.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"

CONFIG="${REPO_ROOT}/wled-config.toml"
EXAMPLE="${REPO_ROOT}/wled-config.toml.example"

die() { echo "[e2e-validate] ERROR: $*" >&2; exit 1; }
warn() { echo "[e2e-validate] WARN: $*" >&2; }
info() { echo "[e2e-validate] INFO: $*"; }

# ── Step 1: Check config exists ─────────────────────────────────────────────

if [[ ! -f "$CONFIG" ]]; then
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║  No wled-config.toml found                                 ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Expected config: $CONFIG"
    echo ""

    if [[ -f "$EXAMPLE" ]]; then
        echo "An example config exists: $EXAMPLE"
        echo ""
        echo "Please copy it and customize for your WLED fleet:"
        echo ""
        echo "  cp wled-config.toml.example wled-config.toml"
        echo "  # Edit wled-config.toml with your device names and IP addresses"
        echo ""
        cat "$EXAMPLE"
    else
        echo "No example config found either."
        echo ""
        echo "Create wled-config.toml with at least one sync group and device."
    fi
    echo ""
    die "Cannot run e2e tests without a valid wled-config.toml"
fi

# ── Step 2: Show current config preview ─────────────────────────────────────

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Current wled-config.toml                                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
cat "$CONFIG"
echo ""

# ── Step 3: Extract device names and addresses from TOML ───────────────────
# We read the file line-by-line to pair each "name" with the next "address"
# that appears in a [[sync_groups.devices]] block.

declare -a DEV_NAMES=()
declare -a DEV_ADDRS=()

in_device_block=false
last_name=""
while IFS= read -r line; do
    # Detect entry into a device block
    if [[ "$line" == *"[[sync_groups.devices]]"* ]]; then
        in_device_block=true
        last_name=""
        continue
    fi
    # Detect entry into anything else (group, schedule, etc.)
    if [[ "$line" == *"[["*"]]"* ]]; then
        in_device_block=false
        continue
    fi

    if [[ "$in_device_block" == "true" ]]; then
        if [[ "$line" =~ ^name[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
            last_name="${BASH_REMATCH[1]}"
        fi
        if [[ "$line" =~ ^address[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
            if [[ -n "$last_name" ]]; then
                DEV_NAMES+=("$last_name")
                DEV_ADDRS+=("${BASH_REMATCH[1]}")
            fi
        fi
    fi
done < "$CONFIG"

device_count=${#DEV_NAMES[@]}
if [[ $device_count -eq 0 ]]; then
    die "No devices found in wled-config.toml"
fi

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Device reachability check                                 ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

all_reachable=true
for i in "${!DEV_NAMES[@]}"; do
    name="${DEV_NAMES[$i]}"
    addr="${DEV_ADDRS[$i]}"
    url="http://${addr}/json/info"

    if curl -s --max-time 3 "$url" >/dev/null 2>&1; then
        echo "  ✅ $name ($addr) — reachable"
    else
        echo "  ❌ $name ($addr) — NOT REACHABLE"
        all_reachable=false
    fi
done

echo ""

if [[ "$all_reachable" == "false" ]]; then
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║  WARNING: Some devices are not responding                  ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    echo "This could mean:"
    echo "  - Devices are powered off or on a different network"
    echo "  - IP addresses in wled-config.toml are wrong"
    echo "  - WLED firmware not running on those devices"
    echo ""
    echo "The smoke test will still run, but expect errors for offline devices."
    echo ""
fi

# ── Step 4: Prompt user for confirmation ─────────────────────────────────────

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Ready to run e2e tests against LIVE WLED hardware           ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "This will send real HTTP requests to the ${device_count} device(s) configured above."
echo "No mutations (power, brightness, color changes) are performed — only read queries."
echo ""

# If not in an interactive terminal, skip the prompt
if [[ ! -t 0 ]]; then
    info "Non-interactive mode — proceeding without confirmation"
    exit 0
fi

read -r -p "Continue? [Y/n] " response
response=${response:-Y}
if [[ ! "$response" =~ ^[Yy]$ ]]; then
    echo "Cancelled by user."
    exit 1
fi

echo ""
