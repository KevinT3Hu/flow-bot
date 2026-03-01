//! Flow-Bot - OneBot-11 Bot Framework with WASM Plugin Support
//!
//! This binary provides a ready-to-use bot that loads configuration from a file
//! and supports hot-reloadable WASM plugins.
//!
//! # Configuration
//!
//! Create a `config.toml` file in the current directory or specify a custom path:
//!
//! ```toml
//! [connection]
//! target = "ws://localhost:19999"
//! auth = "your-auth-token"  # Optional
//!
//! [connection.reconnection]
//! strategy = "infinite"  # or "limited" or "none"
//! initial_delay_ms = 1000
//! max_delay_ms = 60000
//! max_attempts = 10  # Only for "limited" strategy
//!
//! [runtime]
//! enabled = true
//! plugin_dir = "plugins"
//! hot_reload = true
//! reload_debounce_ms = 500
//! max_memory_bytes = 134217728  # 128 MB
//! max_execution_time_ms = 5000
//! enable_wasi = true
//!
//! [logging]
//! level = "info"  # trace, debug, info, warn, error
//! ```
//!
//! # Usage
//!
//! ```bash
//! # Use default config.toml
//! cargo run
//!
//! # Use custom config file
//! cargo run -- --config my-config.toml
//!
//! # Show help
//! cargo run -- --help
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use serde::Deserialize;

use flow_bot::{
    FlowBotBuilder,
    base::connect::{
        ClientConnectionConfig, ConnectionMode, ReconnectionStrategy, ServerConnectionConfig,
    },
    runtime::{FlowBotRuntime, RuntimeConfig},
};

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(name = "flow-bot")]
#[command(about = "OneBot-11 Bot Framework with WASM Plugin Support", long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Override log level (trace, debug, info, warn, error)
    #[arg(short, long)]
    log_level: Option<String>,

    /// Generate a default config file and exit
    #[arg(long)]
    generate_config: bool,
}

/// Main configuration structure
#[derive(Debug, Deserialize)]
struct Config {
    /// Connection configuration
    connection: ConnectionConfig,

    /// Runtime configuration (optional)
    #[serde(default)]
    runtime: RuntimeConfigWrapper,

    /// Logging configuration (optional)
    #[serde(default)]
    logging: LoggingConfig,
}

/// Connection configuration
#[derive(Debug, Deserialize)]
struct ConnectionConfig {
    /// Connection mode: "server" or "client"
    /// - server: bot acts as a server waiting for connections
    /// - client: bot connects to OneBot server (default)
    #[serde(default = "default_mode")]
    mode: String,

    /// WebSocket target URL
    /// - In server mode: the address to bind for incoming connections (e.g., "0.0.0.0:3001")
    /// - In client mode: the OneBot server URL to connect to (e.g., "ws://localhost:3001")
    target: String,

    /// Optional authentication token
    #[serde(default)]
    auth: Option<String>,

    /// Reconnection strategy
    #[serde(default)]
    reconnection: ReconnectionConfig,
}

/// Reconnection configuration
#[derive(Debug, Deserialize)]
#[serde(tag = "strategy", rename_all = "lowercase")]
enum ReconnectionConfig {
    /// Reconnect infinitely with exponential backoff
    Infinite {
        #[serde(default = "default_initial_delay")]
        initial_delay_ms: u64,
        #[serde(default = "default_max_delay")]
        max_delay_ms: u64,
    },
    /// Reconnect for a limited number of attempts
    Limited {
        #[serde(default = "default_max_attempts")]
        max_attempts: u32,
        #[serde(default = "default_initial_delay")]
        initial_delay_ms: u64,
        #[serde(default = "default_max_delay")]
        max_delay_ms: u64,
    },
    /// Do not reconnect
    None,
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self::Infinite {
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
        }
    }
}

impl From<ReconnectionConfig> for ReconnectionStrategy {
    fn from(config: ReconnectionConfig) -> Self {
        match config {
            ReconnectionConfig::Infinite {
                initial_delay_ms,
                max_delay_ms,
            } => ReconnectionStrategy::Infinite {
                initial_delay_ms,
                max_delay_ms,
            },
            ReconnectionConfig::Limited {
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
            } => ReconnectionStrategy::Limited {
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
            },
            ReconnectionConfig::None => ReconnectionStrategy::None,
        }
    }
}

/// Runtime configuration wrapper
#[derive(Debug, Deserialize)]
struct RuntimeConfigWrapper {
    /// Enable WASM plugin runtime
    #[serde(default = "flow_bot::runtime::config::default_true")]
    enabled: bool,

    /// Plugin directory
    #[serde(default = "flow_bot::runtime::config::default_plugin_dir")]
    plugin_dir: PathBuf,

    /// Enable hot reloading
    #[serde(default = "flow_bot::runtime::config::default_true")]
    hot_reload: bool,

    /// Reload debounce delay in milliseconds
    #[serde(default = "flow_bot::runtime::config::default_reload_debounce_ms")]
    reload_debounce_ms: u64,

    /// Maximum memory per plugin (in bytes)
    #[serde(default = "flow_bot::runtime::config::default_max_memory_bytes")]
    max_memory_bytes: usize,

    /// Maximum execution time per event (in milliseconds)
    #[serde(default = "flow_bot::runtime::config::default_max_execution_time_ms")]
    max_execution_time_ms: u64,

    /// Enable WASI support
    #[serde(default = "flow_bot::runtime::config::default_true")]
    enable_wasi: bool,

    /// WASM stack size in bytes
    #[serde(default = "flow_bot::runtime::config::default_wasm_stack_bytes")]
    wasm_stack_bytes: usize,

    /// Request timeout in seconds
    #[serde(default = "flow_bot::runtime::config::default_request_timeout_secs")]
    request_timeout_secs: u64,

    /// Maximum concurrent plugin tasks
    #[serde(default = "flow_bot::runtime::config::default_max_concurrent_plugin_tasks")]
    max_concurrent_plugin_tasks: usize,
}

impl Default for RuntimeConfigWrapper {
    fn default() -> Self {
        Self {
            enabled: flow_bot::runtime::config::default_true(),
            plugin_dir: flow_bot::runtime::config::default_plugin_dir(),
            hot_reload: flow_bot::runtime::config::default_true(),
            reload_debounce_ms: flow_bot::runtime::config::default_reload_debounce_ms(),
            max_memory_bytes: flow_bot::runtime::config::default_max_memory_bytes(),
            max_execution_time_ms: flow_bot::runtime::config::default_max_execution_time_ms(),
            enable_wasi: flow_bot::runtime::config::default_true(),
            wasm_stack_bytes: flow_bot::runtime::config::default_wasm_stack_bytes(),
            request_timeout_secs: flow_bot::runtime::config::default_request_timeout_secs(),
            max_concurrent_plugin_tasks: flow_bot::runtime::config::default_max_concurrent_plugin_tasks(),
        }
    }
}

impl From<RuntimeConfigWrapper> for RuntimeConfig {
    fn from(config: RuntimeConfigWrapper) -> Self {
        RuntimeConfig {
            plugin_dir: config.plugin_dir,
            hot_reload: config.hot_reload,
            reload_debounce_ms: config.reload_debounce_ms,
            max_memory_bytes: config.max_memory_bytes,
            max_execution_time_ms: config.max_execution_time_ms,
            enable_wasi: config.enable_wasi,
            wasm_stack_bytes: config.wasm_stack_bytes,
            request_timeout_secs: config.request_timeout_secs,
            max_concurrent_plugin_tasks: config.max_concurrent_plugin_tasks,
        }
    }
}

/// Logging configuration
#[derive(Debug, Deserialize)]
struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

// Default value functions for connection and logging configuration
fn default_initial_delay() -> u64 {
    1000
}

fn default_max_delay() -> u64 {
    60000
}

fn default_max_attempts() -> u32 {
    10
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_mode() -> String {
    "client".to_string()
}

/// Load configuration from file
fn load_config(path: &PathBuf) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))
}

/// Generate a default configuration file
fn generate_default_config(path: &PathBuf) -> Result<()> {
    let default_config = r#"# Flow-Bot Configuration File
# See https://github.com/KevinT3Hu/flow-bot for documentation

[connection]
# Connection mode: "server" or "client"
# - server: bot acts as a WebSocket server waiting for connections
# - client: bot connects to OneBot server as a WebSocket client (default)
mode = "client"

# WebSocket target URL (required)
# - In server mode: the address to bind for incoming connections (e.g., "0.0.0.0:3001")
# - In client mode: the OneBot server URL to connect to (e.g., "ws://localhost:3001")
target = "ws://localhost:3001"

# Optional authentication token
# auth = "your-auth-token"

# Reconnection strategy configuration
[connection.reconnection]
# Strategy: "infinite", "limited", or "none"
strategy = "infinite"

# Initial delay in milliseconds before first reconnection
initial_delay_ms = 1000

# Maximum delay in milliseconds (exponential backoff cap)
max_delay_ms = 60000

# Maximum reconnection attempts (only for "limited" strategy)
# max_attempts = 10

[runtime]
# Enable WASM plugin runtime
enabled = true

# Directory containing plugin WASM files
plugin_dir = "plugins"

# Enable hot reloading of plugins
hot_reload = true

# Debounce delay in milliseconds for file system events
reload_debounce_ms = 500

# Maximum memory per plugin instance (in bytes)
# Default: 134217728 (128 MB)
max_memory_bytes = 134217728

# Maximum execution time per event handler (in milliseconds)
max_execution_time_ms = 5000

# Enable WASI preview2 support
enable_wasi = true

[logging]
# Log level: trace, debug, info, warn, error
level = "info"
"#;

    std::fs::write(path, default_config)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    println!("Generated default configuration file: {}", path.display());
    Ok(())
}

/// Initialize logging
fn init_logging(level: &str) -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .try_init()
        .map_err(|e| anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Generate config if requested
    if args.generate_config {
        generate_default_config(&args.config)?;
        return Ok(());
    }

    // Load configuration
    let config = load_config(&args.config)?;

    // Initialize logging
    let log_level = args.log_level.as_ref().unwrap_or(&config.logging.level);
    init_logging(log_level)?;

    tracing::info!("Flow-Bot starting...");
    tracing::info!("Configuration file: {}", args.config.display());

    // Create connection config based on mode
    let connection_mode: ConnectionMode = match config.connection.mode.as_str() {
        "server" => ServerConnectionConfig {
            target: config.connection.target.clone(),
            auth: config.connection.auth.clone(),
            reconnection: config.connection.reconnection.into(),
        }
        .into(),
        "client" => ClientConnectionConfig {
            target: config.connection.target.clone(),
            auth: config.connection.auth.clone(),
            reconnection: config.connection.reconnection.into(),
        }
        .into(),
        _ => {
            return Err(anyhow!(
                "Invalid connection mode '{}'. Must be either 'server' or 'client'",
                config.connection.mode
            ));
        }
    };

    tracing::info!("Connection mode: {}", config.connection.mode);
    tracing::info!("Target: {}", config.connection.target);
    if config.connection.auth.is_some() {
        tracing::info!("Authentication: enabled");
    }

    // Build a single FlowBotBuilder whose context is shared with the runtime.
    //
    // Previously the code built a *temporary* bot just to obtain a context,
    // then built a *second* bot for the actual run.  Each call to
    // FlowBotBuilder::build() creates a brand-new Arc<Context>, so the sink
    // that bot.run() stores on the second bot's context was never visible to
    // the plugins, which held a reference to the first (temporary) context.
    // That caused every outbound API call (e.g. send_private_message for !ping)
    // to fail with "No connection".
    //
    // The fix: create one builder, extract its context via builder.context()
    // *before* calling build(), pass that context to the runtime, then attach
    // the runtime and call build() once.  The bot and all plugins share the
    // same Arc<Context>, so when bot.run() stores the WebSocket sink the
    // plugins can use it immediately.
    let mut builder = FlowBotBuilder::new(connection_mode);

    // Create runtime if enabled
    let runtime = if config.runtime.enabled {
        tracing::info!("WASM Plugin Runtime: enabled");
        tracing::info!(
            "  Plugin directory: {}",
            config.runtime.plugin_dir.display()
        );
        tracing::info!("  Hot reload: {}", config.runtime.hot_reload);
        tracing::info!(
            "  Max memory: {} MB",
            config.runtime.max_memory_bytes / 1024 / 1024
        );
        tracing::info!(
            "  Execution timeout: {}ms",
            config.runtime.max_execution_time_ms
        );

        // Obtain the shared context from the builder.  build() will later embed
        // the same Arc<Context> into the FlowBot, guaranteeing one shared sink.
        let bot_context = builder.context();

        let runtime_config: RuntimeConfig = config.runtime.into();
        let runtime = Arc::new(
            FlowBotRuntime::new(runtime_config, bot_context)
                .context("Failed to create WASM runtime")?,
        );

        // Start runtime (load plugins and start watcher)
        runtime.run().await.context("Failed to start runtime")?;

        // Display loaded plugins
        let plugins = runtime.get_plugin_info().await;
        tracing::info!("Loaded {} plugin(s):", plugins.len());
        for plugin in plugins {
            tracing::info!(
                "  - {} v{}: {}",
                plugin.name,
                plugin.version,
                plugin.description
            );
        }

        Some(runtime)
    } else {
        tracing::info!("WASM Plugin Runtime: disabled");
        None
    };

    // Attach the runtime (if any) and build the bot.  builder.context() was
    // already handed to the runtime above, so the bot and all plugins share
    // the same Arc<Context>.
    if let Some(runtime) = runtime.clone() {
        builder = builder.with_runtime(runtime);
    }
    let bot = builder.build();

    tracing::info!("Bot ready! Starting event loop...");

    // Run the bot
    let result = bot.run().await;

    // Cleanup
    if let Some(runtime) = runtime {
        tracing::info!("Shutting down runtime...");
        if let Err(e) = runtime.stop().await {
            tracing::error!("Failed to stop runtime: {}", e);
        }
    }

    // Handle result
    match result {
        Ok(_) => {
            tracing::info!("Bot stopped gracefully");
            Ok(())
        }
        Err(e) => {
            tracing::error!("Bot error: {}", e);
            Err(e.into())
        }
    }
}
