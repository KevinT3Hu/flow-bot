//! # Flow-Bot Plugin SDK
//!
//! A thin wrapper SDK for building Flow-Bot WASM plugins with minimal boilerplate.
//!
//! This crate provides:
//! 1. Pre-generated WIT bindings (no wit-bindgen needed in plugins!)
//! 2. A simple `PluginHandler` trait with typed `Event` parameter
//! 3. Automatic event deserialization from MessagePack
//! 4. Re-exports of `flow-bot-onebot11` and `flow-bot-extractor`
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use flow_bot_plugin_sdk::*;
//!
//! struct MyPlugin;
//!
//! impl PluginHandler for MyPlugin {
//!     fn handle_event(&self, event: Event) -> Result<(), String> {
//!         match event.event {
//!             TypedEvent::Message(msg) => {
//!                 if msg.raw_message.trim() == "!ping" {
//!                     if let Some(group_id) = msg.group_id {
//!                         api::send_group_message(group_id, "Pong!".to_string(), None)?;
//!                     }
//!                 }
//!             }
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//!
//!     fn name(&self) -> &str { "my-plugin" }
//!     fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
//!     fn description(&self) -> &str { "My awesome plugin" }
//! }
//!
//! export_plugin!(MyPlugin);
//! ```

// Pre-generated WIT bindings
#[allow(unused_imports)]
#[doc(hidden)]
pub mod onebot11_plugin;

// Re-export flow-bot crates
pub use flow_bot_extractor as extractor;
pub use flow_bot_onebot11 as onebot11;

pub use onebot11_plugin::flow_bot::onebot11::api;
pub use onebot11_plugin::flow_bot::onebot11::types;

// Re-export serde_json for use in the macro
#[doc(hidden)]
pub use serde_json;

// Re-export specific types for convenience
pub use flow_bot_onebot11::event::message::Message;
pub use flow_bot_onebot11::event::meta_event::MetaEvent;
pub use flow_bot_onebot11::event::notice::Notice;
pub use flow_bot_onebot11::event::request::Request;
pub use flow_bot_onebot11::event::{Event, TypedEvent};

/// Plugin handler trait that users implement
///
/// This trait provides a simpler interface than the WIT-generated `Guest` trait.
/// It receives already-deserialized `Event` objects instead of raw bytes.
///
/// # Example
///
/// ```rust,no_run
/// use flow_bot_plugin_sdk::*;
///
/// struct MyPlugin;
///
/// impl PluginHandler for MyPlugin {
///     fn handle_event(&self, event: Event) -> Result<(), String> {
///         eprintln!("Got event: {:?}", event.event.get_type());
///         Ok(())
///     }
///
///     fn name(&self) -> &str { "my-plugin" }
///     fn version(&self) -> &str { "0.1.0" }
///     fn description(&self) -> &str { "My plugin" }
/// }
/// ```
#[async_trait::async_trait]
pub trait PluginHandler {
    /// Handle an incoming event
    ///
    /// This method is called for each event the bot receives.
    ///
    /// # Arguments
    /// * `event` - The deserialized event
    ///
    /// # Returns
    /// * `Ok(())` - Event handled successfully
    /// * `Err(String)` - Error message if handling failed
    async fn handle_event(&self, event: Event) -> Result<(), String>;

    /// Get the plugin name
    fn name(&self) -> &str;

    /// Get the plugin version
    fn version(&self) -> &str;

    /// Get the plugin description
    fn description(&self) -> &str;
}

/// Export a plugin implementation
///
/// This macro bridges your `PluginHandler` implementation to the pre-generated
/// WIT bindings. It handles automatic event deserialization and delegates to
/// your trait implementation.
///
/// # Requirements
///
/// The plugin handler type must implement both [`PluginHandler`] and [`Default`] traits.
/// The `Default` trait is used to create the singleton handler instance.
///
/// # Limitations
///
/// The handler instance is stored in a static `OnceLock` and lives for the entire duration
/// of the plugin's execution. There is no automatic cleanup mechanism when the plugin is
/// unloaded. If your plugin holds resources that need cleanup (e.g., database connections,
/// file handles), consider using interior mutability patterns or drop guards.
///
/// # Example
///
/// ```rust,ignore
/// use flow_bot_plugin_sdk::*;
///
/// #[derive(Default)]
/// struct MyPlugin;
///
/// impl PluginHandler for MyPlugin {
///     fn handle_event(&self, event: Event) -> Result<(), String> {
///         // Handle event
///         Ok(())
///     }
///
///     fn name(&self) -> &str { "my-plugin" }
///     fn version(&self) -> &str { "0.1.0" }
///     fn description(&self) -> &str { "Description" }
/// }
///
/// export_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! export_plugin {
    ($handler:ty) => {
        // Create a static handler instance that's initialized once
        static __PLUGIN_HANDLER: std::sync::OnceLock<$handler> = std::sync::OnceLock::new();

        // Helper function to get the handler instance
        fn __get_handler() -> &'static $handler {
            __PLUGIN_HANDLER.get_or_init(|| <$handler>::default())
        }

        // Create a bridge struct that implements the WIT Guest trait
        struct __PluginBridge;

        // Implement the WIT Guest trait by delegating to PluginHandler
        impl $crate::onebot11_plugin::exports::flow_bot::onebot11::event_handler::Guest for __PluginBridge {
            async fn handle_event(event_bytes: Vec<u8>) -> Result<(), String> {
                // Deserialize the event from JSON
                let event: $crate::Event = $crate::serde_json::from_slice(&event_bytes)
                    .map_err(|e| format!("Failed to deserialize event: {}", e))?;

                // Get the singleton handler instance and delegate
                let handler = __get_handler();
                $crate::PluginHandler::handle_event(handler, event).await
            }

            async fn plugin_name() -> String {
                let handler = __get_handler();
                $crate::PluginHandler::name(handler).to_string()
            }

            async fn plugin_version() -> String {
                let handler = __get_handler();
                $crate::PluginHandler::version(handler).to_string()
            }

            async fn plugin_desc() -> String {
                let handler = __get_handler();
                $crate::PluginHandler::description(handler).to_string()
            }
        }

        // Export the implementation using the wit-bindgen macro
        $crate::onebot11_plugin::export!(__PluginBridge with_types_in $crate::onebot11_plugin);

        // Re-export the API for convenience
        pub use $crate::api;
        pub use $crate::types;
    };
}

/// Prelude module for convenient imports
///
/// ```rust
/// use flow_bot_plugin_sdk::prelude::*;
/// ```
pub mod prelude {
    //! Convenience re-exports

    pub use crate::extractor;
    pub use crate::onebot11;
    pub use crate::{
        Event, Message, MetaEvent, Notice, PluginHandler, Request, TypedEvent, api, export_plugin,
        types,
    };
}
