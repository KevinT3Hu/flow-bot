use thiserror::Error;

#[derive(Error, Debug)]
pub enum FlowError {
    #[error("Cannot apply extractor {extractor} to event {event}")]
    ExtractorError { extractor: String, event: String },

    #[error("Websocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Ill format message: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("No connection")]
    NoConnection,

    #[error("No response")]
    NoResponse,
}
