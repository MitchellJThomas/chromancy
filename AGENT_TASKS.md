# Agent Tasks — Chromancy Code Smell Fixes

## Agent 1: Telemetry Init Panic Fix ✅
**File:** `src/main.rs`, `src/telemetry.rs`
**Issue:** `telemetry::init()` fallback path panicked after installing stderr subscriber.
**Fix:** Changed `main.rs` to use `match` instead of `unwrap_or_else` + panic. Added `TelemetryGuards::noop()` constructor in `telemetry.rs` that returns empty providers. Server now starts with stderr-only logging when OTLP is unavailable.

## Agent 2: ClientKind Abstraction ✅
**File:** `src/client.rs`
**Issue:** Every public method repeated `match self.inner.as_ref() { ClientKind::Http(i) => ..., ClientKind::Mock(i) => ... }` ~15 times.
**Fix:** Added methods to `ClientKind` enum (`device_name`, `get_state`, `get_info`, `get_full_state`, `list_effects`, `list_palettes`, `get_palette_colors`, `post_state`, `list_presets`, `configure_ntp`, `configure_dusk_schedule`, `raw_request`, `mock_get_state`). All `WledClient` public methods now delegate to `self.inner.method()` instead of inline matching.

**Follow-up — Mock JSON Round-trip:** After review, the Mock backend was applying state mutations directly without going through JSON serialization. This meant Mock tests couldn't catch serialization bugs. Fixed by making `get_state`, `get_info`, and `post_state` on the Mock go through `serde_json::to_value` → `serde_json::from_value` before returning/applying, matching the HTTP path's behavior.

## Agent 3: Retry Logic Enhancement ✅
**File:** `src/client.rs`, `Cargo.toml`
**Issue:** Retry only handled `Network`/`Timeout`, not `Api` errors with 5xx status. No jitter on retry delay.
**Fix:** 
- Added `rand = "0.8"` dependency to `Cargo.toml`
- Added `RETRY_JITTER_MAX_MS = 200` constant
- Added `is_retriable()` helper that returns true for `Network`, `Timeout`, and `Api` with 5xx status
- Added `delay_with_jitter()` using `rand::random::<u64>()` for proper random jitter to avoid thundering herd
- Updated `get_with_retry` and `post_void_with_retry` to use the new helpers

---
**Status:** Complete
**Note:** `cargo check`/`cargo build` could not be run due to network restrictions in the environment (crates.io index unavailable). All changes are syntactically correct by inspection. Run `cargo test` when network is available to verify.
