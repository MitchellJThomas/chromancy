# Contributing to Chromancy

This document describes how to work with the Chromancy codebase, whether you're a human contributor or an AI agent.

## Development Container

Build and run the dev container with Podman:

```bash
podman build -f Dockerfile -t "chromancy-dev"
podman run --rm -i -t -v $PWD:/work localhost/chromancy-dev
```

The container mounts the project directory at `/work` and provides fish shell, Rust toolchain, Node.js, and all build dependencies.

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Run MCP server (for local testing)
cargo run

# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy -- -D warnings
Project Structure


chromancy/
├── CLAUDE.md                 # Project overview (read first!)
├── TASKS.md                  # Current task board
├── INTEGRATION_CONTRACTS.md  # Public API specifications
├── AGENTS.md                 # Agent roles and handoffs
├── CONTRIBUTING.md           # This file
├── Cargo.toml                # Rust dependencies
├── src/
│   ├── main.rs               # MCP server entry point
│   ├── config.rs             # Config file loading
│   ├── client.rs             # WledClient HTTP API
│   ├── sync_group.rs         # WledSyncGroup orchestration
│   ├── fleet.rs              # WledFleet multi-group management
│   ├── tools.rs              # MCP tool definitions
│   ├── types.rs              # Shared data structures
│   ├── error.rs              # WledError enum
│   ├── preset.rs             # Preset operations
│   └── schedule.rs           # NTP/dusk schedule config
├── docs/
│   └── decisions/            # Architecture Decision Records (ADRs)
└── wled-config.toml.example  # Sample configuration
Before You Start

Read CLAUDE.md - Understand project goals and architecture
Read TASKS.md - See what's assigned to your agent/role
Read INTEGRATION_CONTRACTS.md - Know your API boundaries
Check recent commits - See what changed since last session
Development Workflow

Step 1: Pick a Task

Find an unchecked task in TASKS.md assigned to your agent role. If none are assigned, check for blocked tasks (dependencies may be complete now) or ask human for guidance if unclear.

Step 2: Implement

Follow Rust idioms and project conventions
Match the public API in INTEGRATION_CONTRACTS.md
Write tests as you go
Use tracing for logging, not println!
Step 3: Test



# Unit tests
cargo test --lib

# Integration tests (if applicable)
cargo test --test integration

# Ensure no clippy warnings
cargo clippy -- -D warnings
Step 4: Commit


git commit -m "Implement WledClient::set_power() with retry logic

- Adds set_power method to WledClient
- Implements single retry with 500ms delay
- Includes unit test with mock server
- Updates INTEGRATION_CONTRACTS.md"
Step 5: Update Task Board

Edit TASKS.md:

Mark task complete [x]
Add notes about implementation details
Note any breaking changes
Update status if phase is complete
Step 6: Handoff (If Applicable)

If your work unblocks another agent:

Update TASKS.md handoff status
Ensure INTEGRATION_CONTRACTS.md is up to date
Notify next agent (via commit message or human)
Coding Conventions

Error Handling

Always return Result<T, WledError> from fallible operations. Include device name in error context. Use thiserror for error enum derivation.


pub enum WledError {
    #[error("Network error contacting {device}: {source}")]
    Network { device: String, source: reqwest::Error },
    // ...
}
Async Code

All network operations are async
Use tokio runtime
Avoid blocking operations in async context
Documentation

Doc comments on all public functions. Include example usage for complex methods. Link to WLED documentation where relevant.


/// Set the power state of the device.
///
/// This method sends a state update to the WLED device to turn
/// the LEDs on or off. If the device is part of a sync group,
/// the change will propagate to followers via UDP.
///
/// # Arguments
/// * `on` - `true` to turn on, `false` to turn off
///
/// # Example
/// ```no_run
/// let client = WledClient::new("192.168.1.101")?;
/// client.set_power(true).await?;
/// ```
pub async fn set_power(&self, on: bool) -> Result<(), WledError>
Testing

Unit tests in the same file as implementation (#[cfg(test)] module)
Use WledClient::mock() for tests that don't need network
Integration tests in tests/ directory
Test error cases, not just success paths
Breaking Changes

Breaking changes to public APIs require:

Proposal - Open issue or PR describing the change
Review - Human approval + affected agent review
Update - Revise INTEGRATION_CONTRACTS.md
Implement - Make the change
Test - Verify no downstream breakage
Do not break public APIs without coordination.

Security

Never commit wled-config.toml (add to .gitignore)
Config file should be readable only by owner (chmod 600)
No external network access (LAN only)
No logging of sensitive data (IPs, API keys)
Getting Help

Topic	Resource
Architecture questions	docs/decisions/ (ADRs)
API questions	INTEGRATION_CONTRACTS.md
Task questions	TASKS.md or ask human
WLED questions	https://kno.wled.ge
Releasing

Update version in Cargo.toml
Update CHANGELOG.md (if exists)
Tag release: git tag -a v0.1.0 -m "Release v0.1.0"
Push tag: git push origin v0.1.0
Publish to crates.io (if applicable): cargo publish
License

[Add your license here - MIT, Apache 2.0, etc.]



---

There—the whole file is now in a single copyable code block. Now, tell me how you intend to use this project.


