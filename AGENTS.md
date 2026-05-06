# Agent Roles & Handoffs

This document defines agent responsibilities and handoff chains for multi-agent development.

---

## Agent Roles

### Agent 1: Client Layer Owner
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

### Agent 2: Sync Group Layer Owner
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

### Agent 3: MCP Tool Layer Owner
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

### Agent 4: Integration Owner
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
- [ ] MCP server connects to Claude Desktop
- [ ] README is complete
- [ ] Update `TASKS.md` with completion status
- [ ] Notify human for review

---

## Session Protocol

### Starting a Session
1. Read `CLAUDE.md` (project overview)
2. Read `TASKS.md` (current status, your tasks)
3. Read `INTEGRATION_CONTRACTS.md` (your API boundaries)
4. Check recent commits (what changed)

### Ending a Session
1. Run tests (ensure nothing broken)
2. Update `TASKS.md` (mark complete, add notes)
3. Commit working code
4. Note any breaking changes in commit message
5. If handoff-ready: notify next agent

---

## Conflict Resolution

| Issue Type | Resolution |
|------------|------------|
| Interface change | Human review required |
| Implementation detail | Agent discretion |
| Blocked dependency | Update `TASKS.md`, notify human |
| Unclear requirements | Ask human, document decision |

---

## Communication

- **Task status:** `TASKS.md` (source of truth)
- **API changes:** `INTEGRATION_CONTRACTS.md` + PR
- **Architecture decisions:** `docs/decisions/` (ADRs)
- **Blockers:** Update `TASKS.md`, notify human
