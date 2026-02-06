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

/// Currently only WsReverse is supported. I do not intend to implement more but PRs are welcome.
#[derive(Clone)]
pub struct ReverseConnectionConfig {
    pub target: String,
    pub auth: Option<String>,
    pub reconnection: ReconnectionStrategy,
}
