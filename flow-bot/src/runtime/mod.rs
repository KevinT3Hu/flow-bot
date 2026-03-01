//! Runtime module for WASM component model integration
//!
//! This module provides a hot-reloadable WASM plugin system that allows plugins
//! to handle OneBot-11 events. Plugins are loaded from a directory and can be
//! automatically reloaded when their files change.

mod loader;
mod manager;
mod plugin;
pub mod types;
pub mod config;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use wasmtime::component::bindgen;

use crate::base::context::BotContext;

// Generate bindings from WIT interface
bindgen!({
    path: "../wit/onebot11.wit",
    world: "onebot11-plugin",
    imports: {
        default: async
    },
    exports: {
        default: async
    },
});

pub use loader::PluginLoader;
pub use manager::PluginManager;

/// Configuration for the WASM plugin runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Directory containing plugin WASM files
    pub plugin_dir: PathBuf,

    /// Whether to enable hot reloading of plugins
    pub hot_reload: bool,

    /// Debounce delay in milliseconds for file system events
    pub reload_debounce_ms: u64,

    /// Maximum memory per plugin instance (in bytes)
    pub max_memory_bytes: usize,

    /// Maximum execution time per event handler (in milliseconds)
    pub max_execution_time_ms: u64,

    /// Enable WASI preview2 support
    pub enable_wasi: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            plugin_dir: config::default_plugin_dir(),
            hot_reload: config::default_true(),
            reload_debounce_ms: config::default_reload_debounce_ms(),
            max_memory_bytes: config::default_max_memory_bytes(),
            max_execution_time_ms: config::default_max_execution_time_ms(),
            enable_wasi: config::default_true(),
        }
    }
}

/// Main runtime for managing WASM plugins
pub struct FlowBotRuntime {
    config: RuntimeConfig,
    manager: Arc<PluginManager>,
}

impl FlowBotRuntime {
    /// Create a new runtime with the specified configuration
    pub fn new(config: RuntimeConfig, bot_context: BotContext) -> Result<Self> {
        let manager = Arc::new(PluginManager::new(config.clone(), bot_context.clone())?);

        Ok(Self { config, manager })
    }

    /// Create a runtime with default configuration
    pub fn with_plugin_dir(
        plugin_dir: impl Into<PathBuf>,
        bot_context: BotContext,
    ) -> Result<Self> {
        let mut config = RuntimeConfig::default();
        config.plugin_dir = plugin_dir.into();
        Self::new(config, bot_context)
    }

    /// Load all plugins from the plugin directory
    pub async fn load_plugins(&self) -> Result<()> {
        self.manager.load_all_plugins().await
    }

    /// Start the runtime, loading plugins and optionally watching for changes
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting WASM plugin runtime...");
        tracing::info!("Plugin directory: {:?}", self.config.plugin_dir);
        tracing::info!("Hot reload enabled: {}", self.config.hot_reload);

        // Load all plugins initially
        self.load_plugins().await?;

        // Start file watcher if hot reload is enabled
        if self.config.hot_reload {
            tracing::info!("Starting hot reload watcher...");
            self.manager.start_watcher().await?;
        }

        tracing::info!("WASM plugin runtime started successfully");
        Ok(())
    }

    /// Stop the runtime and unload all plugins
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping WASM plugin runtime...");
        self.manager.stop_watcher().await;
        self.manager.unload_all_plugins().await;
        tracing::info!("WASM plugin runtime stopped");
        Ok(())
    }

    /// Handle an event by dispatching it to all loaded plugins
    pub async fn handle_event(&self, event: &[u8]) -> Result<()> {
        self.manager.handle_event(event).await
    }

    /// Get information about all loaded plugins
    pub async fn get_plugin_info(&self) -> Vec<plugin::PluginInfo> {
        self.manager.get_plugin_info().await
    }

    /// Reload a specific plugin by name
    pub async fn reload_plugin(&self, name: &str) -> Result<()> {
        self.manager.reload_plugin(name).await
    }

    /// Unload a specific plugin by name
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        self.manager.unload_plugin(name).await
    }

    /// Get the number of loaded plugins
    pub async fn plugin_count(&self) -> usize {
        self.manager.plugin_count().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::context::Context;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_runtime_creation() {
        let context = Arc::new(Context::new());
        let config = RuntimeConfig {
            plugin_dir: PathBuf::from("/tmp/test_plugins"),
            hot_reload: false,
            ..Default::default()
        };

        let runtime = FlowBotRuntime::new(config, context);
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_default_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.plugin_dir, PathBuf::from("plugins"));
        assert!(config.hot_reload);
        assert_eq!(config.reload_debounce_ms, 500);
    }
}
