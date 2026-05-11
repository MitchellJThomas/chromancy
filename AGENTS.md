# Project Chromancy

A WLED client library and MCP server written in Rust.

## Artistic Practice Context

Chromancy is designed to support **iterative lighting art creation**:

- **Presets are art pieces**, not just scenes
- **Iteration is core**: duplicate → modify → save → compare
- **Playlists matter**: sequences with timing are the primary output
- **Physical coupling**: each preset/playlist pairs with a sculpture

Design decisions should favor:
- Easy preset duplication and naming
- Clear diffing between versions
- Playlist creation and editing
- Metadata tracking (which preset belongs to which artwork)

## Resources

- **WLED Documentation**: https://kno.wled.ge
- **WLED Source Code**: https://github.com/WLED/WLED
- **MCP Specification**: https://modelcontextprotocol.io
- **Qinled Hardware Site**: https://quinled.info/

## Project Goals (Priority Order)

1. **LLM tools for agents** (MCP server implementation)
2. **LLM SDK for agents** (Reusable client library)
3. **Human command lines** (CLI tooling)

---

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│  MCP Client (Claude Desktop / Claude Code / Pi.dev / any client) │
└─────────────────────────────────────────────────────────┘
                          │
                          │ MCP Protocol (stdio)
                          ▼
┌─────────────────────────────────────────────────────────┐
│  wled-mcp-server (Rust)                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │  MCP Tool Layer                                  │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  WledFleet (manages multiple sync groups)        │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  WledSyncGroup (leader + followers)              │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  WledClient (single device HTTP API)             │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                          │
                          │ HTTP
                          ▼
         ┌────────────────┼────────────────┐
         ▼                ▼                ▼
   ┌──────────┐    ┌──────────┐    ┌──────────┐
   │ wled-1   │    │ wled-2   │    │ wled-3   │
   │ (Leader) │    │(Follower)│    │(Follower)│
   │ Group A  │    │ Group A  │    │ Group A  │
   └──────────┘    └──────────┘    └──────────┘
         │
         ▼
   ┌──────────┐    ┌──────────┐    ┌──────────┐
   │ wled-4   │    │ wled-5   │    │ wled-6   │
   │ (Leader) │    │(Follower)│    │(Follower)│
   │ Group B  │    │ Group B  │    │ Group B  │
   └──────────┘    └──────────┘    └──────────┘
```

---

## Module Structure

```
src/
├── main.rs           # MCP server bootstrap, tool registration
├── config.rs         # Config file loading, SyncGroupConfig structs
├── client.rs         # WledClient - HTTP API client for single device
├── sync_group.rs     # WledSyncGroup - orchestrates leader + followers
├── fleet.rs          # WledFleet - manages multiple sync groups
├── tools.rs          # MCP tool definitions and handlers
├── types.rs          # Shared data structures (WledState, etc.)
├── error.rs          # WledError enum with context
├── preset.rs         # Preset operations (list, activate, save)
└── schedule.rs       # NTP/dusk schedule configuration
```

---

## Core Design Goals

1. **Mirror WLED's JSON API** closely (makes docs/debugging easier)
2. **Rust-idiomatic** (Result types, builder pattern where useful)
3. **Async-first** (all network calls are non-blocking)
4. **Clear error types** (distinguish network vs. API vs. config errors)
5. **Extensible** (new WLED features = new methods, not rewrites)
6. **Multi-group support** (devices organized into independent sync groups)
7. **Leader/follower terminology** (no master/slave language)

---

## Core Types

### `WledClient` (Single Device)

| Category | Methods |
|----------|---------|
| **Construction** | `new()`, `builder()` |
| **Queries** | `get_state()`, `get_info()`, `get_full_state()`, `list_effects()`, `list_palettes()`, `get_palette_colors()`, `ping()` |
| **Mutations** | `set_power()`, `set_brightness()`, `set_color()`, `set_effect()`, `set_palette()`, `set_transition()`, `set_state()` |
| **Presets** | `list_presets()`, `activate_preset()`, `activate_preset_by_name()`, `save_preset()`, `delete_preset()` |
| **Schedule** | `configure_ntp()`, `configure_dusk_schedule()` |
| **Escape Hatch** | `raw_request()` |

### `WledSyncGroup` (Leader + Followers)

| Category | Methods |
|----------|---------|
| **Group Ops** | `activate_preset()`, `set_power()`, `set_brightness()`, `get_state()` |
| **Device Access** | `leader()`, `get_follower()`, `list_followers()` |
| **Sync Health** | `check_sync_health()`, `force_resync()` |

### `WledFleet` (Multiple Sync Groups)

| Category | Methods |
|----------|---------|
| **Loading** | `load_from_config()` |
| **Group Access** | `list_groups()`, `get_group()`, `get_group_for_device()` |
| **Device Access** | `get_device()` (searches all groups) |
| **Fleet Ops** | `activate_preset_broadcast()`, `get_fleet_status()`, `check_all_sync_health()` |

---

## MCP Tool Definitions

### Group Management

| Tool | Parameters | Description |
|------|------------|-------------|
| `list_groups` | none | Return all sync group names |
| `list_devices` | `group_name: Option<String>` | Return devices in a group, or all devices |

### Device Queries

| Tool | Parameters | Description |
|------|------------|-------------|
| `get_device_info` | `device_name: String` | Get device capabilities (LED count, firmware, uptime) |
| `get_device_state` | `device_name: String` | Get current state (power, brightness, effect, palette) |
| `get_group_status` | `group_name: String` | Get status of a specific sync group |
| `get_fleet_status` | none | Get status of entire fleet (all groups) |

### Group Control

| Tool | Parameters | Description |
|------|------------|-------------|
| `activate_preset` | `group_name: String, preset_name: String` | Activate preset on group (leader syncs to followers) |
| `activate_preset_broadcast` | `group_names: Vec<String>, preset_name: String` | Activate same preset across multiple groups |
| `list_presets` | `group_name: String` | List available presets on group's leader |
| `set_power` | `group_name: String, on: bool` | Turn group on/off via leader |
| `set_brightness` | `group_name: String, brightness: u8` | Set brightness (0-255) across group |
| `set_color` | `group_name: String, r: u8, g: u8, b: u8` | Set color on group leader's primary segment |
| `set_effect` | `group_name: String, effect_name: String` | Set effect by name on group leader |
| `set_palette` | `group_name: String, palette_name: String` | Set palette by name on group leader |
| `set_channel_color` | `group_name: String, channel: u8, r: u8, g: u8, b: u8` | Set color on specific Dig-Quad channel (leader only) |

### Sync Health & Troubleshooting

| Tool | Parameters | Description |
|------|------------|-------------|
| `check_sync_health` | `group_name: Option<String>` | Report sync drift (specific group or all) |
| `force_resync` | `group_name: String` | Force followers to re-sync with leader |
| `get_individual_state` | `device_name: String` | Get state of single device (troubleshooting) |
| `set_individual_power` | `device_name: String, on: bool` | Control single device independently |

---

## Configuration

### `wled-config.toml`

```toml
# Define multiple sync groups
[[sync_groups]]
name = "main_house"

[[sync_groups.devices]]
name = "wled-1"
address = "192.168.1.101"
is_leader = true
device_type = "DigQuad"

[[sync_groups.devices]]
name = "wled-2"
address = "192.168.1.102"
is_leader = false
device_type = "DigUno"

[[sync_groups.devices]]
name = "wled-3"
address = "192.168.1.103"
is_leader = false
device_type = "Dig2Go"

[[sync_groups]]
name = "patio"

[[sync_groups.devices]]
name = "wled-4"
address = "192.168.1.104"
is_leader = true
device_type = "DigUno"

[[sync_groups.devices]]
name = "wled-5"
address = "192.168.1.105"
is_leader = false
device_type = "Dig2Go"

[[sync_groups.devices]]
name = "wled-6"
address = "192.168.1.106"
is_leader = false
device_type = "Dig2Go"

[schedule]
enabled = true
dusk_preset = "Evening Mode"
off_time = "00:30"
```

---

## Error Handling

### `WledError` Variants

```rust
pub enum WledError {
    Network { device: String, source: reqwest::Error },
    Api { device: String, status: u16, message: String },
    DeviceNotFound(String),
    PresetNotFound(String),
    SyncDrift { device: String, expected: String, actual: String },
    InvalidChannel { device: String, channel: u8, max_channels: u8 },
    ConfigError(String),
    Timeout,
}
```

**Key principle:** All errors include device name context for debugging multi-device setups.

---

## Security Considerations

- **No external exposure**: Server runs locally, only talks to your LAN
- **Config file permissions**: Keep `wled-config.toml` readable only by you (`chmod 600`)
- **Optional auth**: WLED supports optional API keys—implement when needed
- **Read-only mode**: Consider a flag that disables all mutation operations

---

## Implementation Notes

### Dependencies

```toml
[dependencies]
rmcp = "0.1"                    # MCP protocol
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Technical Requirements

- Use `rmcp` crate for MCP protocol implementation
- Use `reqwest` for HTTP client with async runtime (tokio)
- Use `serde`/`serde_json` for JSON serialization
- Use `thiserror` for error enum derivation
- Use `tracing` for structured logging
- Default timeout: 5 seconds per HTTP request
- Retry failed requests once with 500ms delay before surfacing error
- Cache `list_effects()` and `list_palettes()` results (TTL: 1 hour)
- Do NOT cache state queries (always fresh)

### Testing

- Unit tests: Mock HTTP responses for client methods
- Provide `WledClient::mock()` constructor for unit testing
- Integration tests: User will test against real hardware

---

## Key Design Decisions

1. **Fleet abstraction**: `WledFleet` is the top-level type passed to MCP tool handlers
2. **Group-aware tools**: Most tools accept `group_name` parameter (explicit is better than implicit)
3. **Leader-centric group ops**: Group operations go through leader; WLED UDP sync handles followers
4. **Individual access preserved**: Any device can be accessed directly by name for troubleshooting
5. **Sync health monitoring**: Detect drift between leader and followers; offer force-resync
6. **Config-driven**: Device addresses, names, and group membership loaded from TOML at startup
7. **Structured errors**: All errors include device name context for debugging

---

## Future Considerations (Not MVP)

- WebSocket subscription for real-time state change notifications
- mDNS device discovery
- Multi-segment fluent builder API
- Preset schedule management via MCP (vs. WLED's built-in scheduler)
- Device belonging to multiple groups (keep it simple for MVP: one group per device)

---

## Multi-Agent Coordination

### Agent Roles

#### Agent 1: Client Layer Owner
**Scope:** `src/client.rs`, `src/error.rs`, `src/types.rs`
**Dependencies:** None
**Handoff to:** Agent 2

**Responsibilities:**
- Implement `WledClient` HTTP API
- Define `WledError` error types
- Define core types (`WledState`, `WledInfo`, etc.)
- Write unit tests with mocked responses
- Provide `WledClient::mock()` for testing

**Handoff Checklist:**
- [ ] Public API matches `INTEGRATION_CONTRACTS.md`
- [ ] All methods return `Result<T, WledError>`
- [ ] Unit tests pass
- [ ] Update `TASKS.md` with completion status
- [ ] Notify Agent 2

---

#### Agent 2: Sync Group Layer Owner
**Scope:** `src/sync_group.rs`, `src/fleet.rs`
**Dependencies:** Agent 1's `WledClient`
**Handoff to:** Agent 3

**Responsibilities:**
- Implement `WledSyncGroup` orchestration
- Implement `WledFleet` multi-group management
- Sync health detection and force-resync
- Integration tests with `WledClient`

**Handoff Checklist:**
- [ ] Uses only public `WledClient` API
- [ ] Group operations route through leader
- [ ] Individual device access works
- [ ] Integration tests pass
- [ ] Update `TASKS.md` with completion status
- [ ] Notify Agent 3

---

#### Agent 3: MCP Tool Layer Owner
**Scope:** `src/tools.rs`, `src/main.rs`
**Dependencies:** Agent 2's `WledFleet`
**Handoff to:** Agent 4

**Responsibilities:**
- Define MCP tools with `rmcp` crate
- Implement tool handlers
- Parameter validation
- Response formatting
- Tool handler unit tests

**Handoff Checklist:**
- [ ] All tools defined per `INTEGRATION_CONTRACTS.md`
- [ ] Errors translate to `McpError` correctly
- [ ] Tool responses are JSON-serializable
- [ ] Unit tests pass
- [ ] Update `TASKS.md` with completion status
- [ ] Notify Agent 4

---

#### Agent 4: Integration Owner
**Scope:** `src/main.rs`, config loading, CLI, README
**Dependencies:** All agents

**Responsibilities:**
- Binary entry point
- Config file loading (`wled-config.toml`)
- CLI commands (optional)
- Logging setup
- README documentation
- End-to-end testing coordination

**Handoff Checklist:**
- [ ] Binary builds and runs
- [ ] Config loading works
- [ ] MCP server starts and accepts connections
- [ ] README is complete
- [ ] Update `TASKS.md` with completion status
- [ ] Notify human for review

---

### Session Protocol

#### Starting a Session
1. Read `AGENTS.md` (project overview and agent roles)
2. Read `TASKS.md` (current status, your tasks)
3. Read `INTEGRATION_CONTRACTS.md` (your API boundaries)
4. Check recent commits (what changed)

#### Ending a Session
1. Run tests (ensure nothing broken)
2. Update `TASKS.md` (mark complete, add notes)
3. Commit working code
4. Note any breaking changes in commit message
5. If handoff-ready: notify next agent

---

### Conflict Resolution

| Issue Type | Resolution |
|------------|------------|
| Interface change | Human review required |
| Implementation detail | Agent discretion |
| Blocked dependency | Update `TASKS.md`, notify human |
| Unclear requirements | Ask human, document decision |

---

### Communication

- **Task status:** `TASKS.md` (source of truth)
- **API changes:** `INTEGRATION_CONTRACTS.md` + PR
- **Architecture decisions:** `docs/decisions/` (ADRs)
- **Blockers:** Update `TASKS.md`, notify human
