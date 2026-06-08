use crate::error::FlowError;

/// Control flow signal returned by a handler or service.
///
/// - `Continue` — pass the event to the next handler.
/// - `Block` — stop processing; do not pass to subsequent handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerControl {
    Continue,
    Block,
}

/// Error type indicating that a handler should be skipped.
///
/// Returning `Err(HandlerError)` from a handler is semantically equivalent to the old
/// `HandlerControl::Skip` — the event is passed to the next handler.
///
/// The optional message is stored as a `Box<str>` for efficiency: it is more compact
/// than `String` (no capacity field) and avoids allocation when `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerError {
    message: Option<Box<str>>,
}

impl HandlerError {
    /// Create a new `HandlerError` with a descriptive message.
    pub fn new(msg: impl Into<Box<str>>) -> Self {
        Self {
            message: Some(msg.into()),
        }
    }

    /// Create a `HandlerError` representing a silent skip (no message).
    pub fn skip() -> Self {
        Self { message: None }
    }

    /// Returns the error message, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.message {
            Some(msg) => write!(f, "handler skipped: {}", msg),
            None => write!(f, "handler skipped"),
        }
    }
}

impl std::error::Error for HandlerError {}

impl From<FlowError> for HandlerError {
    fn from(err: FlowError) -> Self {
        Self::new(err.to_string())
    }
}

impl From<serde_json::Error> for HandlerError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Helper trait to normalize both `HandlerControl` and `Result<HandlerControl, HandlerError>`
/// into a single result type. Used by the `#[flow_service]` macro.
pub trait IntoHandlerResult {
    fn into_result(self) -> Result<HandlerControl, HandlerError>;
}

impl IntoHandlerResult for HandlerControl {
    fn into_result(self) -> Result<HandlerControl, HandlerError> {
        Ok(self)
    }
}

impl IntoHandlerResult for Result<HandlerControl, HandlerError> {
    fn into_result(self) -> Result<HandlerControl, HandlerError> {
        self
    }
}
