# Chromancy Task Board

## Phase 1: Core Client Layer
**Owner:** Agent 1
**Status:** COMPLETE (2026-04-23)
**Handoff to:** Agent 2

### Tasks
- [x] `WledClient` struct with builder pattern
- [x] `WledClient::new(address)` constructor
- [x] `WledClient::builder()` for advanced config
- [x] HTTP GET methods: `get_state()`, `get_info()`, `get_full_state()`
- [x] HTTP POST methods: `set_state()`, `set_power()`, `set_brightness()`, `set_color()`
- [x] Query methods: `list_effects()`, `list_palettes()`, `get_palette_colors()`, `ping()`
- [x] Escape hatch: `raw_request()`
- [x] `WledError` enum with all variants
- [x] `WledState`, `WledStateRequest`, `WledInfo` type definitions
- [x] `Segment`, `SegmentRequest` type definitions
- [x] Unit tests with mocked HTTP responses (13 tests, all passing)
- [x] `WledClient::mock()` constructor for testing
- [x] `NtpConfig` with typed `NtpServer` (Hostname | Ipv4)
- [x] `DuskScheduleConfig` for schedule operations
- [x] Preset operations: `list_presets`, `activate_preset`, `activate_preset_by_name`, `save_preset`, `delete_preset`

### Acceptance Criteria
- [x] All methods return `Result<T, WledError>`
- [x] Errors include device name context
- [x] Mock mode works without network
- [x] 5-second default timeout
- [x] Single retry with 500ms delay on failure

### Files
- `src/error.rs` — WledError enum
- `src/types.rs` — all data types
- `src/client.rs` — WledClient implementation + tests
- `src/lib.rs` — module exports
- `src/main.rs` — minimal binary placeholder

---

## Phase 2: Sync Group Layer
**Owner:** Agent 2
**Status:** COMPLETE (2026-04-23)
**Handoff to:** Agent 3

### Tasks
- [x] `WledSyncGroup` struct (leader + followers)
- [x] `WledSyncGroup::new()` + `add_follower()` constructors
- [x] Group operations: `activate_preset()`, `set_power()`, `set_brightness()`, `get_state()`, `set_color()`, `set_effect()`, `set_palette()`, `set_channel_color()`
- [x] Device access: `leader()`, `get_follower()`, `list_followers()`, `get_device()`, `list_devices()`
- [x] Sync health: `check_sync_health()`, `force_resync()`
- [x] `WledFleet` struct (insertion-order groups via `indexmap`)
- [x] `WledFleet::load_from_config()` + `from_config()` TOML parsing
- [x] Fleet operations: `list_groups()`, `get_group()`, `get_group_for_device()`, `get_device()`, `list_all_devices()`
- [x] Broadcast: `activate_preset_broadcast()`, `get_fleet_status()`, `check_all_sync_health()`
- [x] Config validation (zero/multiple leaders caught at load time)
- [x] 21 new tests (14 sync_group + 7 fleet), all passing

### Acceptance Criteria
- [x] Group ops route through leader only
- [x] Individual device access works for any follower
- [x] Sync drift detection compares leader vs follower preset (`ps`)
- [x] Config file loading validates required fields

### Files
- `src/config.rs` — FleetConfig, SyncGroupConfig, DeviceConfig, ScheduleConfig
- `src/sync_group.rs` — WledSyncGroup + tests
- `src/fleet.rs` — WledFleet + tests

---

## Phase 3: MCP Tool Layer
**Owner:** Agent 3
**Status:** COMPLETE (2026-04-24)
**Handoff to:** Agent 4

### Tasks
- [x] MCP server bootstrap (`main.rs`)
- [x] Tool registration with `rmcp` crate (`#[tool(tool_box)]` macro)
- [x] Group management tools: `list_groups`, `list_devices`
- [x] Device query tools: `get_device_info`, `get_device_state`, `get_group_status`, `get_fleet_status`
- [x] Group control tools: `activate_preset`, `activate_preset_broadcast`, `list_presets`, `set_power`, `set_brightness`, `set_color`, `set_effect`, `set_palette`, `set_channel_color`
- [x] Sync health tools: `check_sync_health`, `force_resync`
- [x] Individual control tools: `get_individual_state`, `set_individual_power`
- [x] Parameter validation for all tools
- [x] Response formatting (consistent JSON / plain-text)
- [x] OpenTelemetry (traces, metrics, logs) via OTLP gRPC — OTLP-compatible (grafana/otel-lgtm)
- [x] `src/telemetry.rs` — OTel init, `Metrics` instruments, `TelemetryGuards` shutdown
- [x] `docker-compose.lgtm.yml — Grafana LGTM all-in-one stack for local OTel validation
- [x] All 35 tests passing, zero warnings

### Acceptance Criteria
- [x] All tools accept required parameters
- [x] Errors translate to `String` Err (MCP-safe)
- [x] Tool responses are JSON-serializable
- [x] No panics—graceful error handling throughout
- [x] OTel emits to OTLP (port 4317) — SigNoz receives traces, metrics, logs

---

## Phase 4: Integration & CLI
**Owner:** Agent 4
**Status:** Ready (Phase 3 complete)  
**Dependencies:** All phases complete

### Tasks
- [ ] `main.rs` entry point with MCP server startup
- [ ] Config file loading from `wled-config.toml`
- [ ] CLI commands (optional: `chromancy status`, `chromancy preset <name>`)
- [ ] Logging setup with `tracing` and `tracing-subscriber`
- [ ] Example config file (`wled-config.toml.example`)
- [ ] README with setup instructions
- [ ] README with Claude Desktop configuration
- [ ] End-to-end testing against real hardware (user)
- [ ] Security review (config permissions, no external exposure)

### Acceptance Criteria
- [ ] Binary builds and runs
- [ ] MCP server connects to Claude Desktop
- [ ] All tools respond correctly
- [ ] Config errors are clear and actionable
- [ ] README is complete and tested

---

## Notes

- Update this file at the end of each agent session
- Mark tasks complete as they're done
- Add new tasks if scope changes
- Note any breaking changes in handoff comments
