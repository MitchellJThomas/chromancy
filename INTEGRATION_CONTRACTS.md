# Integration Contracts

This document defines the public APIs that agents depend on. Changes to these interfaces require review and coordination with downstream agents.

---

## Phase 1: WledClient Public API

### Constructor
```rust
pub struct WledClient { /* opaque */ }

impl WledClient {
    pub fn new(address: impl Into<String>) -> Result<Self, WledError>;
    
    pub fn builder(address: impl Into<String>) -> WledClientBuilder;
    
    pub fn mock() -> WledClientMockBuilder;  // For testing
}

