# ADR 001: MCP Server Architecture

**Date:** 2026-04-14  
**Status:** Accepted  
**Author:** Mitch (with Confer)  
**Deciders:** Mitch  

---

## Context

Project Chromancy needs to expose WLED lighting control to LLM agents. The primary use case is an MCP (Model Context Protocol) server for Claude Desktop/Claude Code. Secondary use cases include a reusable Rust SDK and a human-facing CLI.

The system must support:
- Multiple WLED devices (6+ QuinLED units)
- UDP sync groups with leader/follower topology
- Individual device control for troubleshooting
- Multi-group support (e.g., "main_house", "patio")
- Local-only network access (security)

---

## Decision

We will implement a **three-layer architecture**:

┌─────────────────────────────────────────┐ │ MCP Tool Layer (tools.rs, main.rs) │ │ - Tool definitions & handlers │ │ - Parameter validation │ │ - Response formatting │ └─────────────────────────────────────────┘ │ depends on ▼ ┌─────────────────────────────────────────┐ │ Fleet Layer (fleet.rs, sync_group.rs) │ │ - WledFleet (multiple sync groups) │ │ - WledSyncGroup (leader + followers) │ │ - Sync health monitoring │ └─────────────────────────────────────────┘ │ depends on ▼ ┌─────────────────────────────────────────┐ │ Client Layer (client.rs, types.rs) │ │ - WledClient (single device HTTP) │ │ - WledError (error types) │ │ - WledState, WledInfo (data types) │ └─────────────────────────────────────────┘



### Key Architectural Choices

1. **Interface-First Development**
   - Public APIs defined in `INTEGRATION_CONTRACTS.md` before implementation
   - Enables parallel multi-agent development
   - Changes require review and coordination

2. **Leader-Centric Group Operations**
   - Group commands route through leader device only
   - WLED's UDP sync handles follower synchronization
   - Reduces network chatter, matches WLED's design

3. **Individual Device Access Preserved**
   - Any device can be accessed directly by name
   - Enables troubleshooting when sync drifts
   - Supports special cases (e.g., one-off color changes)

4. **Config-Driven Group Membership**
   - `wled-config.toml` defines groups, leaders, followers
   - Loaded at startup by `WledFleet::load_from_config()`
   - No runtime group reconfiguration (MVP scope)

5. **Structured Error Handling**
   - `WledError` includes device name context
   - Distinguishes network vs. API vs. config errors
   - Enables clear debugging in multi-device setups

6. **MCP Tools Map to Fleet Operations**
   - Most tools accept `group_name` parameter
   - Some tools support broadcast across groups
   - Individual control tools for troubleshooting

---

## Consequences

### Positive

- **Clear separation of concerns**: Each layer has a single responsibility
- **Testable**: Client layer can be mocked; sync group tested in isolation
- **Extensible**: New WLED features add methods, not rewrites
- **Multi-agent friendly**: Agents can work on different layers in parallel
- **Matches WLED's mental model**: Leader/follower sync is first-class

### Negative

- **Complexity**: Three layers is more code than a flat structure
- **Learning curve**: New contributors must understand the abstraction
- **Performance**: Extra abstraction layer adds minimal overhead (acceptable)

### Risks

| Risk | Mitigation |
|------|------------|
| Interface drift between layers | `INTEGRATION_CONTRACTS.md` as source of truth |
| Sync group assumptions wrong | Test against real hardware early |
| MCP tool parameters too rigid | Use `Option<T>` for optional params, iterate based on usage |

---

## Compliance

All implementation must conform to:
- `CLAUDE.md` - Project overview and goals
- `INTEGRATION_CONTRACTS.md` - Public API specifications
- `TASKS.md` - Task assignments and acceptance criteria
- `AGENTS.md` - Agent roles and handoff protocols

---

## Notes

This architecture is designed for MVP scope. Future extensions (WebSocket subscriptions, mDNS discovery, dynamic group switching) should be added as new modules without breaking existing interfaces.
