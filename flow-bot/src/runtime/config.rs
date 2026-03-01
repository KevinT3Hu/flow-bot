//! Default configuration values for the WASM plugin runtime
//!
//! This module provides centralized default value functions to avoid duplication
//! between the library's RuntimeConfig and the binary's configuration parsing.

use std::path::PathBuf;

/// Default value for boolean flags that should be true
pub fn default_true() -> bool {
    true
}

/// Default plugin directory path
pub fn default_plugin_dir() -> PathBuf {
    PathBuf::from("plugins")
}

/// Default reload debounce delay in milliseconds
pub fn default_reload_debounce_ms() -> u64 {
    500
}

/// Default maximum memory per plugin in bytes (128 MB)
pub fn default_max_memory_bytes() -> usize {
    128 * 1024 * 1024
}

/// Default maximum execution time per event in milliseconds (5 seconds)
pub fn default_max_execution_time_ms() -> u64 {
    5000
}

/// Default WASM stack size in bytes (1 MB)
pub fn default_wasm_stack_bytes() -> usize {
    1024 * 1024
}

/// Default request timeout in seconds (30 seconds)
pub fn default_request_timeout_secs() -> u64 {
    30
}

/// Default maximum concurrent plugin tasks (10)
pub fn default_max_concurrent_plugin_tasks() -> usize {
    10
}
