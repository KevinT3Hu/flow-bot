//! Plugin manager for handling multiple WASM plugins
//!
//! This module provides the PluginManager which orchestrates loading, reloading,
//! and unloading multiple plugins, as well as file watching for hot reload.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as AnyhowContext, Result, anyhow};
use dashmap::DashMap;
use notify_debouncer_mini::{
    DebouncedEvent, DebouncedEventKind, Debouncer, new_debouncer,
    notify::{Error as NotifyError, RecommendedWatcher, RecursiveMode},
};
use tokio::sync::{Mutex, RwLock};

use crate::base::context::BotContext;
use crate::runtime::RuntimeConfig;
use crate::runtime::loader::{LoadedPlugin, PluginLoader};
use crate::runtime::plugin::PluginInfo;

/// Manager for all loaded plugins
pub struct PluginManager {
    config: RuntimeConfig,
    loader: Arc<PluginLoader>,
    plugins: Arc<DashMap<String, Arc<Mutex<LoadedPlugin>>>>,
    watcher: Arc<RwLock<Option<Debouncer<RecommendedWatcher>>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: RuntimeConfig, bot_context: BotContext) -> Result<Self> {
        let loader = Arc::new(PluginLoader::new(config.clone(), bot_context.clone())?);

        Ok(Self {
            config,
            loader,
            plugins: Arc::new(DashMap::new()),
            watcher: Arc::new(RwLock::new(None)),
        })
    }

    /// Load all plugins from the plugin directory
    pub async fn load_all_plugins(&self) -> Result<()> {
        tracing::info!("Loading all plugins from {:?}", self.config.plugin_dir);

        let plugin_paths = self
            .loader
            .scan_plugin_directory()
            .context("Failed to scan plugin directory")?;

        if plugin_paths.is_empty() {
            tracing::warn!("No plugins found in {:?}", self.config.plugin_dir);
            return Ok(());
        }

        let mut loaded_count = 0;
        let mut failed_count = 0;

        for path in plugin_paths {
            match self.load_plugin_internal(&path).await {
                Ok(_) => loaded_count += 1,
                Err(e) => {
                    failed_count += 1;
                    tracing::error!("Failed to load plugin {:?}: {:?}", path, e);
                }
            }
        }

        tracing::info!(
            "Plugin loading complete: {} loaded, {} failed",
            loaded_count,
            failed_count
        );

        Ok(())
    }

    /// Load a single plugin from a path
    async fn load_plugin_internal(&self, path: &Path) -> Result<()> {
        let plugin = self
            .loader
            .load_plugin(path)
            .await
            .with_context(|| format!("Failed to load plugin from {:?}", path))?;

        let plugin_name = plugin.name.clone();

        // Check if plugin with this name already exists
        if self.plugins.contains_key(&plugin_name) {
            tracing::warn!(
                "Plugin {} already loaded, replacing with version from {:?}",
                plugin_name,
                path
            );
            self.plugins.remove(&plugin_name);
        }

        self.plugins
            .insert(plugin_name.clone(), Arc::new(Mutex::new(plugin)));

        tracing::info!("Plugin {} loaded successfully", plugin_name);
        Ok(())
    }

    /// Unload all plugins
    pub async fn unload_all_plugins(&self) {
        tracing::info!("Unloading all plugins");
        self.plugins.clear();
        tracing::info!("All plugins unloaded");
    }

    /// Reload a specific plugin by name
    pub async fn reload_plugin(&self, name: &str) -> Result<()> {
        tracing::info!("Reloading plugin: {}", name);

        let plugin_entry = self
            .plugins
            .get(name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", name))?;

        let plugin_arc = plugin_entry.value().clone();
        let mut plugin = plugin_arc.lock().await;

        plugin
            .reload(&self.loader)
            .await
            .with_context(|| format!("Failed to reload plugin: {}", name))?;

        tracing::info!("Plugin {} reloaded successfully", name);
        Ok(())
    }

    /// Reload a plugin by its file path
    pub async fn reload_plugin_by_path(&self, path: &Path) -> Result<()> {
        tracing::info!("Reloading plugin from path: {:?}", path);

        // Find the plugin with this path
        for entry in self.plugins.iter() {
            let plugin_arc = entry.value().clone();
            let plugin = plugin_arc.lock().await;

            if plugin.path == path {
                let name = plugin.name.clone();
                drop(plugin); // Release lock before reloading
                return self.reload_plugin(&name).await;
            }
        }

        // If not found, try to load as new plugin
        tracing::info!("Plugin not currently loaded, loading as new: {:?}", path);
        self.load_plugin_internal(path).await
    }

    /// Unload a specific plugin by name
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        tracing::info!("Unloading plugin: {}", name);

        self.plugins
            .remove(name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", name))?;

        tracing::info!("Plugin {} unloaded successfully", name);
        Ok(())
    }

    /// Handle an event by dispatching it to all loaded plugins
    pub async fn handle_event(&self, event: &[u8]) -> Result<()> {
        if self.plugins.is_empty() {
            tracing::debug!("No plugins loaded, skipping event");
            return Ok(());
        }

        tracing::debug!("Dispatching event to {} plugins", self.plugins.len());

        let mut handles = Vec::new();

        // Collect all plugins and spawn concurrent tasks
        for entry in self.plugins.iter() {
            let plugin_arc = entry.value().clone();
            let event_data = event.to_vec();

            let handle = tokio::spawn(async move {
                let mut plugin = plugin_arc.lock().await;
                let plugin_name = plugin.name.clone();

                match plugin.handle_event(&event_data).await {
                    Ok(_) => {
                        tracing::debug!("Plugin {} handled event successfully", plugin_name);
                        Ok(())
                    }
                    Err(e) => {
                        tracing::warn!("Plugin {} failed to handle event: {}", plugin_name, e);
                        Err(e)
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all plugins to complete
        let results = futures::future::join_all(handles).await;

        let mut success_count = 0;
        let mut error_count = 0;

        for result in results {
            match result {
                Ok(Ok(_)) => success_count += 1,
                Ok(Err(_)) => error_count += 1,
                Err(e) => {
                    error_count += 1;
                    tracing::error!("Plugin task panicked: {}", e);
                }
            }
        }

        tracing::debug!(
            "Event dispatch complete: {} succeeded, {} failed",
            success_count,
            error_count
        );

        Ok(())
    }

    /// Get information about all loaded plugins
    pub async fn get_plugin_info(&self) -> Vec<PluginInfo> {
        let mut infos = Vec::new();

        for entry in self.plugins.iter() {
            let plugin = entry.value().lock().await;
            infos.push(PluginInfo::new(
                plugin.name.clone(),
                plugin.version.clone(),
                plugin.description.clone(),
                plugin.path.clone(),
            ));
        }

        infos.sort_by(|a, b| a.name.cmp(&b.name));
        infos
    }

    /// Get the number of loaded plugins
    pub async fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Start the file watcher for hot reloading
    pub async fn start_watcher(&self) -> Result<()> {
        if !self.config.hot_reload {
            return Ok(());
        }

        let mut watcher_guard = self.watcher.write().await;

        if watcher_guard.is_some() {
            tracing::warn!("File watcher already running");
            return Ok(());
        }

        let plugin_dir = self.config.plugin_dir.clone();
        let manager = Arc::new(self.create_weak_ref());
        let debounce_duration = Duration::from_millis(self.config.reload_debounce_ms);

        let mut debouncer = new_debouncer(
            debounce_duration,
            move |result: Result<Vec<DebouncedEvent>, NotifyError>| {
                if let Ok(events) = result {
                    let manager_clone = manager.clone();
                    tokio::spawn(async move {
                        for event in events {
                            if let DebouncedEventKind::Any = event.kind {
                                let path = &event.path;
                                if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                                    tracing::info!("Detected change in plugin: {:?}", path);
                                    if let Err(e) = manager_clone.reload_plugin_by_path(path).await
                                    {
                                        tracing::error!(
                                            "Failed to reload plugin {:?}: {}",
                                            path,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    });
                }
            },
        )
        .context("Failed to create file watcher")?;

        debouncer
            .watcher()
            .watch(&plugin_dir, RecursiveMode::NonRecursive)
            .with_context(|| format!("Failed to watch directory: {:?}", plugin_dir))?;

        *watcher_guard = Some(debouncer);

        tracing::info!("File watcher started for {:?}", plugin_dir);
        Ok(())
    }

    /// Stop the file watcher
    pub async fn stop_watcher(&self) {
        let mut watcher_guard = self.watcher.write().await;
        if watcher_guard.is_some() {
            *watcher_guard = None;
            tracing::info!("File watcher stopped");
        }
    }

    /// Create a weak reference wrapper for use in callbacks
    fn create_weak_ref(&self) -> PluginManagerRef {
        PluginManagerRef {
            loader: self.loader.clone(),
            plugins: self.plugins.clone(),
        }
    }
}

/// A weak reference to PluginManager components for use in callbacks
#[derive(Clone)]
struct PluginManagerRef {
    loader: Arc<PluginLoader>,
    plugins: Arc<DashMap<String, Arc<Mutex<LoadedPlugin>>>>,
}

impl PluginManagerRef {
    async fn reload_plugin_by_path(&self, path: &Path) -> Result<()> {
        tracing::info!("Reloading plugin from path: {:?}", path);

        // Find the plugin with this path
        for entry in self.plugins.iter() {
            let plugin_arc = entry.value().clone();
            let plugin = plugin_arc.lock().await;

            if plugin.path == path {
                let name = plugin.name.clone();
                drop(plugin); // Release lock

                let plugin_entry = self
                    .plugins
                    .get(&name)
                    .ok_or_else(|| anyhow!("Plugin not found: {}", name))?;

                let plugin_arc = plugin_entry.value().clone();
                let mut plugin = plugin_arc.lock().await;

                return plugin
                    .reload(&self.loader)
                    .await
                    .with_context(|| format!("Failed to reload plugin: {}", name));
            }
        }

        // If not found, try to load as new plugin
        tracing::info!("Plugin not currently loaded, loading as new: {:?}", path);
        let plugin = self.loader.load_plugin(path).await?;
        let plugin_name = plugin.name.clone();

        self.plugins
            .insert(plugin_name.clone(), Arc::new(Mutex::new(plugin)));

        tracing::info!("Plugin {} loaded successfully", plugin_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::context::Context;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_manager_creation() {
        let context = Arc::new(Context::new());
        let config = RuntimeConfig {
            plugin_dir: PathBuf::from("/tmp/test_plugins"),
            hot_reload: false,
            ..Default::default()
        };

        let manager = PluginManager::new(config, context);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_count() {
        let context = Arc::new(Context::new());
        let config = RuntimeConfig {
            plugin_dir: PathBuf::from("/tmp/test_plugins"),
            hot_reload: false,
            ..Default::default()
        };

        let manager = PluginManager::new(config, context).unwrap();
        let count = manager.plugin_count().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_load_all_plugins_empty_dir() {
        let context = Arc::new(Context::new());
        let config = RuntimeConfig {
            plugin_dir: PathBuf::from("/nonexistent/plugins"),
            hot_reload: false,
            ..Default::default()
        };

        let manager = PluginManager::new(config, context).unwrap();
        let result = manager.load_all_plugins().await;
        assert!(result.is_ok());
    }
}
