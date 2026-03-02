//! Plugin loader for WASM components
//!
//! This module handles loading WASM component files, creating instances,
//! and managing the WASM runtime engine.

use std::path::{Path, PathBuf};

use anyhow::{Context as AnyhowContext, Result, anyhow};
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;

use crate::base::context::BotContext;
use crate::runtime::plugin::PluginState;
use crate::runtime::{Onebot11Plugin, RuntimeConfig};

/// WASM plugin loader responsible for loading and instantiating plugins
pub struct PluginLoader {
    engine: Engine,
    config: RuntimeConfig,
    bot_context: BotContext,
}

impl PluginLoader {
    /// Create a new plugin loader with the given configuration
    pub fn new(config: RuntimeConfig, bot_context: BotContext) -> Result<Self> {
        let engine = Self::create_engine(&config)?;

        Ok(Self {
            engine,
            config,
            bot_context,
        })
    }

    /// Create a configured WASM engine
    fn create_engine(config: &RuntimeConfig) -> Result<Engine> {
        let mut wasm_config = Config::new();

        // Enable component model
        wasm_config.wasm_component_model(true);

        // Set memory limits from configuration
        wasm_config.max_wasm_stack(config.wasm_stack_bytes as usize);

        // Enable optimizations
        wasm_config.cranelift_opt_level(wasmtime::OptLevel::Speed);

        // Enable epoch-based interruption for timeouts
        wasm_config.epoch_interruption(true);

        Engine::new(&wasm_config).map_err(|e| anyhow!(e).context("Failed to create WASM engine"))
    }

    /// Load a plugin from a WASM file
    pub async fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin> {
        tracing::info!("Loading plugin from: {:?}", path);

        // Validate file exists and has .wasm extension
        if !path.exists() {
            return Err(anyhow!("Plugin file does not exist: {:?}", path));
        }

        if path.extension().and_then(|s| s.to_str()) != Some("wasm") {
            return Err(anyhow!("Plugin file must have .wasm extension: {:?}", path));
        }

        // Load the component
        let component = Component::from_file(&self.engine, path).map_err(|e| {
            anyhow!(e).context(format!("Failed to load WASM component from {:?}", path))
        })?;

        // Create linker and add WASI
        let mut linker = Linker::new(&self.engine);

        if self.config.enable_wasi {
            wasmtime_wasi::p2::add_to_linker_async(&mut linker)
                .map_err(|e| anyhow!(e).context("Failed to add WASI to linker"))?;
        }

        // Add host API implementation to linker so plugins can call imported functions
        crate::runtime::Onebot11Plugin::add_to_linker::<_, HasSelf<PluginState>>(
            &mut linker,
            |state: &mut PluginState| state,
        )
        .map_err(|e| anyhow!(e).context("Failed to add OneBot11 API to linker"))?;

        // Create store with plugin state
        let mut store = self.create_store(path)?;

        // Instantiate the plugin
        let instance = Onebot11Plugin::instantiate_async(&mut store, &component, &linker)
            .await
            .map_err(|e| anyhow!(e).context("Failed to instantiate plugin"))?;

        // Get plugin metadata
        let plugin_name = instance
            .flow_bot_onebot11_event_handler()
            .call_plugin_name(&mut store)
            .await?;

        let plugin_version = instance
            .flow_bot_onebot11_event_handler()
            .call_plugin_version(&mut store)
            .await?;
        let plugin_desc = instance
            .flow_bot_onebot11_event_handler()
            .call_plugin_desc(&mut store)
            .await?;

        tracing::info!(
            "Plugin loaded: {} v{} - {}",
            plugin_name,
            plugin_version,
            plugin_desc
        );

        instance
            .flow_bot_onebot11_event_handler()
            .call_init(&mut store)
            .await?
            .map_err(|e| anyhow!("Plugin {} init failed: {}", plugin_name, e))?;

        Ok(LoadedPlugin {
            name: plugin_name,
            version: plugin_version,
            description: plugin_desc,
            path: path.to_path_buf(),
            instance,
            store,
            engine: self.engine.clone(),
        })
    }

    /// Create a store with plugin state and WASI context
    fn create_store(&self, plugin_path: &Path) -> Result<Store<PluginState>> {
        // Build WASI context
        let mut wasi_builder = WasiCtxBuilder::new();

        // Allow plugins to inherit stdin/stdout/stderr
        wasi_builder.inherit_stdio();

        // Set environment variables
        if let Some(plugin_name) = plugin_path.file_stem() {
            wasi_builder.env("PLUGIN_NAME", plugin_name.to_string_lossy());
        }
        wasi_builder.env("PLUGIN_PATH", plugin_path.to_string_lossy());

        let wasi = wasi_builder.build();

        let mut plugin_state = PluginState::new(self.bot_context.clone(), wasi);
        plugin_state.set_max_execution_time_ms(self.config.max_execution_time_ms);

        let mut store = Store::new(&self.engine, plugin_state);

        // Set execution timeout using epoch interruption
        // Each epoch is 10ms, so divide max_execution_time_ms by 10 to get epochs
        let timeout_epochs = self.config.max_execution_time_ms / 10;
        store.set_epoch_deadline(timeout_epochs);
        store.epoch_deadline_async_yield_and_update(timeout_epochs);

        Ok(store)
    }

    /// Scan the plugin directory for WASM files
    pub fn scan_plugin_directory(&self) -> Result<Vec<PathBuf>> {
        let plugin_dir = &self.config.plugin_dir;

        if !plugin_dir.exists() {
            tracing::warn!("Plugin directory does not exist: {:?}", plugin_dir);
            return Ok(Vec::new());
        }

        if !plugin_dir.is_dir() {
            return Err(anyhow!("Plugin path is not a directory: {:?}", plugin_dir));
        }

        let mut plugins = Vec::new();

        for entry in std::fs::read_dir(plugin_dir)
            .with_context(|| format!("Failed to read plugin directory: {:?}", plugin_dir))?
        {
            let entry = entry.with_context(|| "Failed to read directory entry")?;
            let path = entry.path();

            if path.is_file()
                && path.extension().and_then(|s: &std::ffi::OsStr| s.to_str()) == Some("wasm")
            {
                plugins.push(path);
            }
        }

        tracing::info!("Found {} plugin files in {:?}", plugins.len(), plugin_dir);
        Ok(plugins)
    }

    /// Get the engine for creating new stores
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}

/// A loaded plugin instance with its store and metadata
pub struct LoadedPlugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
    pub instance: Onebot11Plugin,
    pub store: Store<PluginState>,
    pub engine: Engine,
}

impl LoadedPlugin {
    /// Handle an event with this plugin
    pub async fn handle_event(&mut self, event: &[u8]) -> Result<()> {
        // Set epoch deadline for timeout
        let timeout_epochs = self.store.data().max_execution_time_ms() / 10;
        self.store.set_epoch_deadline(timeout_epochs);

        self.instance
            .flow_bot_onebot11_event_handler()
            .call_handle_event(&mut self.store, event)
            .await
            .map_err(|e| anyhow!(e).context("Failed to handle event with plugin"))?
            .map_err(|e| anyhow!("Plugin {} returned an error: {}", self.name, e))?;

        Ok(())
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        self.instance
            .flow_bot_onebot11_event_handler()
            .call_cleanup(&mut self.store)
            .await
            .map_err(|e| anyhow!(e).context("Failed to call plugin cleanup"))?
            .map_err(|e| anyhow!("Plugin {} cleanup returned an error: {}", self.name, e))?;

        Ok(())
    }

    /// Reload this plugin from disk
    pub async fn reload(&mut self, loader: &PluginLoader) -> Result<()> {
        tracing::info!("Reloading plugin: {}", self.name);

        let new_plugin = loader.load_plugin(&self.path).await?;

        // Replace the instance and store
        self.instance = new_plugin.instance;
        self.store = new_plugin.store;
        self.version = new_plugin.version;
        self.description = new_plugin.description;
        self.engine = new_plugin.engine;

        tracing::info!("Plugin {} reloaded successfully", self.name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::context::Context;
    use std::sync::Arc;

    #[test]
    fn test_engine_creation() {
        let config = RuntimeConfig::default();
        let result = PluginLoader::create_engine(&config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_loader_creation() {
        let config = RuntimeConfig::default();
        let context = Arc::new(Context::default());
        let loader = PluginLoader::new(config, context);
        assert!(loader.is_ok());
    }

    #[test]
    fn test_scan_nonexistent_directory() {
        let config = RuntimeConfig {
            plugin_dir: PathBuf::from("/nonexistent/path"),
            ..Default::default()
        };
        let context = Arc::new(Context::default());
        let loader = PluginLoader::new(config, context).unwrap();
        let result = loader.scan_plugin_directory();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
