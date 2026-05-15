---
name: e2e-mcp-testing
description: Run end-to-end smoke tests against a live WLED fleet via the Chromancy MCP server. Use when validating changes to the MCP tool layer, HTTP client, WLED JSON deserialization, or sync-group orchestration against real hardware. Also use after adding new devices to wled-config.toml or before cutting a release.
---

# E2E MCP Testing for Chromancy

These scripts exercise the Chromancy MCP server against **live WLED hardware**.
They complement the wiremock-based unit tests (`cargo test`) by validating real
HTTP responses, JSON deserialization quirks (RGBW colors, firmware variations),
and multi-device fleet behavior.

## Prerequisites

- `cargo build` has produced `./target/debug/chromancy`
- `wled-config.toml` exists in the repo root and defines at least one sync group
- Node.js is available (the harness uses `child_process` to drive the MCP stdio server)

## Quick Start

From the repo root:

```bash
# Full read-only smoke test — all MCP tools with valid + invalid arguments
./.pi/skills/e2e-mcp-testing/scripts/run-smoke-test.sh

# Quick per-device health check
./.pi/skills/e2e-mcp-testing/scripts/run-device-check.sh
```

## What happens when you run a test

Both scripts perform a **validation and confirmation flow** before talking to hardware:

1. **Check `wled-config.toml` exists**
   - If missing, the script shows the example config (`wled-config.toml.example`) and instructs you to copy and customize it.
2. **Preview the config**
   - Prints the full contents of `wled-config.toml` so you can verify it's the correct one.
3. **Device reachability check**
   - Does a quick `curl` to `http://<address>/json/info` for every device.
   - Shows ✅/❌ for each device so you know if any are offline.
4. **User confirmation prompt**
   - In interactive mode, asks you to confirm before sending requests to live hardware.
   - In non-interactive mode (CI), auto-continues.
5. **Run the Node.js harness**
   - Executes the actual smoke test or health check.

## Scripts

### `run-smoke-test.sh`

Runs `tests/e2e/mcp-readonly-smoke-test.mjs`, which:
1. Starts `./target/debug/chromancy wled-config.toml`
2. Initializes the MCP stdio connection
3. Calls every read-only MCP tool with both valid and invalid arguments
4. Prints all JSON responses so you can spot errors, crashes, or unexpected formats

Use this after any change touching:
- `src/tools.rs` (MCP tool handlers)
- `src/sync_group.rs` (group orchestration)
- `src/client.rs` (HTTP client / deserialization)
- `src/types.rs` (serde models)

### `run-device-check.sh`

Runs `tests/e2e/mcp-device-health-check.mjs`, which:
1. Starts `./target/debug/chromancy wled-config.toml`
2. Calls `get_device_info`, `get_device_state`, and `get_individual_state` on every device in the fleet
3. Reports whether each device is reachable and whether JSON parsed successfully

Use this when:
- Adding new devices to `wled-config.toml`
- Debugging a device that appears unresponsive
- Checking firmware differences across your fleet
- Validating RGBW vs RGB color array handling on mixed hardware

### `validate-config.sh` (used internally)

Shared helper that both scripts call. You can run it standalone to preview your
config and check device reachability without running the full test harness:

```bash
./.pi/skills/e2e-mcp-testing/scripts/validate-config.sh
```

## Interpreting Output

- `<<< OK (...)` — Tool returned parseable JSON. Check fields match expectations.
- `<<< ERROR: ...` — Tool returned an error JSON object. May indicate device offline, unknown preset, missing group, etc.
- `JSON decode error` or `RPC ERROR` — MCP protocol or serde issue. Usually means a code regression.
- `[stderr] Network error ... error decoding response body` — The most common live-device crash. Typically means a field in `src/types.rs` doesn't match the actual WLED JSON (e.g., RGBW 4-element colors, unexpected object types).

## Extending the Tests

The Node.js harness scripts live in `tests/e2e/`:
- `tests/e2e/mcp-readonly-smoke-test.mjs` — Full smoke test
- `tests/e2e/mcp-device-health-check.mjs` — Per-device health check
- `tests/e2e/README.md` — Full documentation for the harness

To add a new test case, edit the `.mjs` file and add a `callTool()` invocation.
No rebuild is required — the scripts drive the already-built MCP binary over stdio.
