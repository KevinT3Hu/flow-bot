# Comprehensive Code Review Report for flow-bot

**Date:** 2026-03-01  
**Project:** flow-bot - OneBot-11 Bot Framework with WASM Plugin Support  
**Version Reviewed:** 0.2.5

---

## 2. Potential Bugs

### 2.1 Error Handling Issues

| Location | Issue | Impact |
|----------|-------|--------|
| `flow-bot/src/lib.rs:333` | `auth.parse().unwrap()` | Panic if auth header contains invalid characters |
| `flow-bot/src/lib.rs:399` | `serde_json::from_str(msg).unwrap()` | **Panic on any invalid JSON message** - will crash the bot |
| `flow-bot/src/runtime/loader.rs:150-152` | Epoch deadline always set to 1 | Timeout mechanism may not work correctly; should calculate based on `max_execution_time_ms` |

### 2.2 Logic Issues

**Reconnection Counter Bug** (`lib.rs:213-219`, `265-270`):
The `reconnect_attempt` counter is reset to 0 in `run_once()` when connection succeeds, but the `run_with_*_reconnect` methods increment it AFTER a successful connection closes. 

**More Critical**: In `run_with_limited_reconnect`, the check `if attempt >= max_attempts` happens BEFORE the attempt, but the counter is incremented AFTER. This allows one extra attempt.

### 2.3 Resource Leaks

**Plugin Task Panic Handling** (`manager.rs:213-216`):
If a plugin task panics, the error is logged but continues. However, if many plugins panic, this could exhaust the async runtime.

---

## 3. TODO / Toy Implementations

### 3.1 Unimplemented Features

| Feature | Location | Notes |
|---------|----------|-------|
| `RecordFormat` deserialization | `api/mod.rs:171-182` | Only has `Serialize`, no `Deserialize` - will fail if bot receives this type |
| Message content extraction | `runtime/types.rs:220-221, 229-230` | Converts rich messages to plain text - loses all formatting |
| `get_group_member_info` in Host | `plugin.rs:341-352` | Has workaround comment: "Call send_obj directly because ApiExt has wrong return type" |

### 3.2 Incomplete Implementations

**Server Mode Auth** (`lib.rs:309-312`):
```rust
// Optionally validate auth header from the request
// Note: tokio_tungstenite's accept_async doesn't provide easy access to headers
// For more sophisticated auth, we'd need to implement a custom accept
// For now, we accept the connection and auth validation can be done at the protocol level
```
