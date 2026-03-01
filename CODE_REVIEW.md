# Comprehensive Code Review Report for flow-bot

**Date:** 2026-03-01  
**Project:** flow-bot - OneBot-11 Bot Framework with WASM Plugin Support  
**Version Reviewed:** 0.2.5

---

### 2.2 Logic Issues

**Reconnection Counter Bug** (`lib.rs:213-219`, `265-270`):
The `reconnect_attempt` counter is reset to 0 in `run_once()` when connection succeeds, but the `run_with_*_reconnect` methods increment it AFTER a successful connection closes. 

**More Critical**: In `run_with_limited_reconnect`, the check `if attempt >= max_attempts` happens BEFORE the attempt, but the counter is incremented AFTER. This allows one extra attempt.

---

## 3. TODO / Toy Implementations

### 3.1 Unimplemented Features

| Feature | Location | Notes | Status |
|---------|----------|-------|--------|
| `RecordFormat` deserialization | `api/mod.rs:171-182` | Only has `Serialize`, no `Deserialize` - will fail if bot receives this type | ✅ **FIXED**: Added `Deserialize` derive |
| Message content extraction | `runtime/types.rs:220-221, 229-230` | Converts rich messages to plain text - loses all formatting | ✅ **FIXED**: Now uses JSON serialization to preserve full message structure |
| `get_group_member_info` in Host | `plugin.rs:341-352` | Has workaround comment: "Call send_obj directly because ApiExt has wrong return type" | ✅ **FIXED**: Removed workaround, now uses `ApiExt` trait methods directly |

### 3.2 Incomplete Implementations

**Server Mode Auth** (`lib.rs:309-312`):
```rust
// Optionally validate auth header from the request
// Note: tokio_tungstenite's accept_async doesn't provide easy access to headers
// For more sophisticated auth, we'd need to implement a custom accept
// For now, we accept the connection and auth validation can be done at the protocol level
```
