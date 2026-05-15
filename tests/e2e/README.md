# End-to-End Integration Tests

These Node.js scripts exercise the Chromancy MCP server against a **live WLED fleet**.
They complement the wiremock-based unit tests in `src/` by validating real HTTP
responses from actual hardware.

## Requirements

- Node.js (any recent version)
- A running WLED fleet defined in `wled-config.toml` at the repo root
- The debug binary built: `cargo build`

## Scripts

### `mcp-readonly-smoke-test.mjs`

Comprehensive smoke test of all read-only MCP tools.

```bash
node tests/e2e/mcp-readonly-smoke-test.mjs
```

What it does:
- Initializes the MCP stdio connection
- Lists all registered tools
- Calls every read-only tool (`list_groups`, `list_devices`, `get_device_info`,
  `get_device_state`, `get_group_status`, `get_fleet_status`, `list_presets`,
  `check_sync_health`, `get_individual_state`) with both valid and invalid arguments
- Prints JSON responses so you can spot errors, crashes, or unexpected formats

Use this after any code change that touches:
- `src/tools.rs` (MCP tool handlers)
- `src/sync_group.rs` (group orchestration)
- `src/client.rs` (HTTP client / deserialization)
- `src/types.rs` (serde models)

### `mcp-device-health-check.mjs`

Quick per-device sanity check.

```bash
node tests/e2e/mcp-device-health-check.mjs
```

What it does:
- Calls `get_device_info`, `get_device_state`, and `get_individual_state` on every
  device in the fleet
- Reports whether each device is reachable and whether the response JSON parsed
  successfully

Use this when:
- Adding new devices to `wled-config.toml`
- Debugging a device that appears unresponsive
- Checking firmware differences across your fleet
