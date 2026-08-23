use thiserror::Error;

#[derive(Error, Debug)]
pub enum FlowError {
    #[error("Cannot apply extractor {extractor} to event {event}")]
    ExtractorError { extractor: String, event: String },

    #[error("Invalid authorization header")]
    InvalidAuth,

    #[error("Websocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Http error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Ill format message: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("No connection")]
    NoConnection,

    #[error("No response")]
    NoResponse,

    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    #[error("Reconnection failed after {0} attempts")]
    ReconnectionFailed(u32),

    #[error("Api call failed: status={status}, retcode={retcode}, message={message:?}")]
    ApiError {
        status: String,
        retcode: i32,
        message: Option<String>,
    },

    #[error("Invalid connection config: {0}")]
    InvalidConfig(String),

    #[error("The active connection type cannot call OneBot APIs")]
    ApiUnavailable,

    #[error("Operation requires a message event, got a {0} event")]
    NotAMessageEvent(&'static str),

    #[error("Server error: {0}")]
    ServerError(#[from] std::io::Error),

    #[cfg(feature = "turso")]
    #[error("Turso error: {0}")]
    TursoError(#[from] turso::Error),
}
