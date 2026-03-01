/// Reconnection strategy configuration
#[derive(Clone, Debug)]
pub enum ReconnectionStrategy {
    /// Reconnect endlessly with exponential backoff
    Infinite {
        /// Initial delay in milliseconds (default: 1000)
        initial_delay_ms: u64,
        /// Maximum delay in milliseconds (default: 60000)
        max_delay_ms: u64,
    },
    /// Reconnect for a limited number of attempts
    Limited {
        /// Maximum number of reconnection attempts
        max_attempts: u32,
        /// Initial delay in milliseconds (default: 1000)
        initial_delay_ms: u64,
        /// Maximum delay in milliseconds (default: 60000)
        max_delay_ms: u64,
    },
    /// Do not reconnect
    None,
}

impl Default for ReconnectionStrategy {
    fn default() -> Self {
        Self::Infinite {
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
        }
    }
}

/// Connection mode enum to support both server and client websocket modes
#[derive(Clone, Debug)]
pub enum ConnectionMode {
    /// Server mode: bot acts as a WebSocket server waiting for OneBot client connections
    Server(ServerConnectionConfig),
    /// Client mode: bot acts as a WebSocket client connecting to a OneBot server
    Client(ClientConnectionConfig),
}

impl ConnectionMode {
    /// Get the reconnection strategy for this connection mode
    pub fn reconnection_strategy(&self) -> &ReconnectionStrategy {
        match self {
            ConnectionMode::Server(config) => &config.reconnection,
            ConnectionMode::Client(config) => &config.reconnection,
        }
    }
}

/// Server WebSocket connection configuration
/// In this mode, the bot acts as a server waiting for OneBot client connections
#[derive(Clone, Debug)]
pub struct ServerConnectionConfig {
    /// WebSocket server bind address (e.g., "0.0.0.0:3001")
    pub target: String,
    /// Optional authentication token (validated from client requests)
    pub auth: Option<String>,
    /// Reconnection strategy
    pub reconnection: ReconnectionStrategy,
}

/// Client WebSocket connection configuration
/// In this mode, the bot acts as a client connecting to a OneBot server
#[derive(Clone, Debug)]
pub struct ClientConnectionConfig {
    /// WebSocket server URL to connect to (e.g., "ws://localhost:3001")
    pub target: String,
    /// Optional authentication token
    pub auth: Option<String>,
    /// Reconnection strategy
    pub reconnection: ReconnectionStrategy,
}
