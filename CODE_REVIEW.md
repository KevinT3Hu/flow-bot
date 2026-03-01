# Comprehensive Code Review Report for flow-bot

**Date:** 2026-03-01  
**Project:** flow-bot - OneBot-11 Bot Framework with WASM Plugin Support  
**Version Reviewed:** 0.2.5

---

## 1. Code Optimization Opportunities

### 1.1 Performance Optimizations

| Location | Issue | Severity | Recommendation |
|----------|-------|----------|----------------|
| `flow-bot/src/lib.rs:264` | Exponential backoff overflow risk | Medium | Use `checked_pow` or `saturating_mul` to prevent potential overflow when `reconnect_attempt` is large |
| `flow-bot/src/runtime/manager.rs:177-200` | Spawning task per plugin per event | Medium | Consider using a bounded channel or thread pool; current design may exhaust resources under high load with many plugins |
| `flow-bot/src/base/context.rs:97` | Fixed 30-second timeout | Low | Make timeout configurable via `RuntimeConfig` |
| `flow-bot/src/runtime/loader.rs:44` | 1MB WASM stack hardcoded | Low | Move to configuration |

### 1.2 Code Style Improvements

**Duplicate Default Values**: The main binary (`main.rs`) and `runtime/mod.rs` both define identical default functions:

```rust
// main.rs:251-289
fn default_true() -> bool { true }
fn default_plugin_dir() -> PathBuf { PathBuf::from("plugins") }
// ... etc
```

**Recommendation**: Move all default value functions to a shared `config` module.

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

### 2.4 Type Safety Issues

**Wrong Return Type in Trait** (`api_ext.rs:293-294`):
```rust
async fn get_group_member_info(...) -> Result<GroupInfoResponse, Self::Error>
```

Should return `GroupMemberInfo`, not `GroupInfoResponse`. The implementation in `api_impl.rs:248` also has this issue:

```rust
async fn get_group_member_info(...) -> Result<crate::api::GroupInfoResponse, Self::Error> {
    impl_api!(self, get_group_member_info, group_id, user_id, no_cache)
}
```

This is a bug - the return type doesn't match what the API actually returns.

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

**WIT Interface Version** (`wit/onebot11.wit:1`):
```
package flow-bot:onebot11@0.1.0;
```
The version is 0.1.0 but the crates are at 0.2.5 - version mismatch.

---

## 4. Hardcoded Configurations

### 4.1 Magic Numbers

| Value | Location | Description | Should Be |
|-------|----------|-------------|-----------|
| `128 * 1024 * 1024` | `main.rs:263` | Max memory (128MB) | Configurable |
| `5000` | `main.rs:267` | Max execution time (5s) | Configurable |
| `500` | `main.rs:259` | Reload debounce (ms) | Configurable |
| `1024 * 1024` | `loader.rs:44` | WASM stack size (1MB) | Configurable |
| `30` | `context.rs:97` | Request timeout (seconds) | Configurable |
| `10` | `loader.rs:210` | Epoch timeout calculation | Based on `max_execution_time_ms` |

### 4.2 File Paths

| Path | Location | Issue |
|------|----------|-------|
| `"plugins"` | `main.rs:256` | Default plugin dir (also in `runtime/mod.rs:60`) |
| `"plugins-dist"` | `config.example.toml:45` | Example uses different default! |
| `/tmp/test_plugins` | `manager.rs:385, 412` | Hardcoded in tests (acceptable) |

**Inconsistency**: The example config says `plugin_dir = "plugins-dist"` but the code defaults to `"plugins"`.

---

## 5. Additional Observations

### 5.1 Dependency Version Inconsistencies

- `Cargo.toml` uses `serde = "1.0.228"` but `flow-bot-onebot11/Cargo.toml` uses same - OK
- `rust-toolchain.toml` specifies nightly features but file is empty/unread

### 5.2 Unused Dependencies

- `flow-bot/Cargo.toml`: `config = "0.15.19"` - Not used (custom config parsing in `main.rs`)
- `flow-bot/Cargo.toml`: `reqwest` - Not obviously used in core code
- `flow-bot/Cargo.toml`: `rmp-serde` - Listed but MessagePack not used (JSON used instead)

### 5.3 Feature Flag Inconsistency

`flow-bot-onebot11` has `api` feature for `async-trait`, but `flow-bot-extractor` always includes `async-trait` without feature gate.

---

## Summary

### Critical Issues:
1. `unwrap()` on JSON parsing in `check_is_echo()` will crash the bot on invalid input
2. Wrong return type in `get_group_member_info` trait definition
3. Plugin directory mismatch between example config and code defaults

### High Priority:
1. Add overflow protection to exponential backoff
2. Make hardcoded timeouts configurable
3. Fix epoch deadline calculation for WASM plugins

### Medium Priority:
1. Reduce code duplication in defaults
2. ~~Add input validation for connection mode~~ (Fixed)
3. ~~Document the `Default` requirement for `export_plugin!` macro~~ (Fixed)

### Low Priority:
1. Move generated WIT bindings out of VCS
2. Unify dependency versions across workspace
3. Add `Deserialize` to `RecordFormat`
