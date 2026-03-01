//! Log collector for the web interface
//!
//! This module provides a `tracing_subscriber::Layer` that captures log events
//! and broadcasts them to connected web UI clients via a tokio broadcast channel.

use std::fmt;

use tokio::sync::broadcast;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// Log message format for the web UI
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LogMessage {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

impl LogMessage {
    /// Create a new log message
    pub fn new(level: &str, target: &str, message: &str) -> Self {
        use chrono::Utc;
        Self {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            target: target.to_string(),
            message: message.to_string(),
        }
    }
}

/// A tracing layer that broadcasts log events to the web UI
pub struct WebLogLayer {
    sender: broadcast::Sender<LogMessage>,
}

impl WebLogLayer {
    /// Create a new WebLogLayer with the given broadcast sender
    pub fn new(sender: broadcast::Sender<LogMessage>) -> Self {
        Self { sender }
    }

    /// Get a clone of the sender for use elsewhere
    pub fn sender(&self) -> broadcast::Sender<LogMessage> {
        self.sender.clone()
    }
}

impl<S> Layer<S> for WebLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Get the log level
        let level = match event.metadata().level() {
            &Level::TRACE => "TRACE",
            &Level::DEBUG => "DEBUG",
            &Level::INFO => "INFO",
            &Level::WARN => "WARN",
            &Level::ERROR => "ERROR",
        };

        // Get the target (module path)
        let target = event.metadata().target();

        // Format the message
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message.unwrap_or_default();

        // Create the log message
        let log_msg = LogMessage::new(level, target, &message);

        // Broadcast to all connected clients (ignore errors if no receivers)
        let _ = self.sender.send(log_msg);
    }
}

/// Visitor to extract the message from a tracing event
#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        // The message field is typically named "message"
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

/// Initialize the web log layer and return the broadcast sender
///
/// This should be called during application startup to set up log broadcasting.
/// The returned sender can be passed to the web server.
pub fn init_web_log_layer() -> (WebLogLayer, broadcast::Sender<LogMessage>) {
    let (tx, _rx) = broadcast::channel(1000);
    let layer = WebLogLayer::new(tx.clone());
    (layer, tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_message_creation() {
        let msg = LogMessage::new("INFO", "test_module", "Test message");
        assert_eq!(msg.level, "INFO");
        assert_eq!(msg.target, "test_module");
        assert_eq!(msg.message, "Test message");
        // Timestamp should be set
        assert!(!msg.timestamp.is_empty());
    }

    #[test]
    fn test_web_log_layer_creation() {
        let (tx, _rx) = broadcast::channel(100);
        let layer = WebLogLayer::new(tx);
        let sender = layer.sender();
        // Should be able to create a receiver from the sender
        let _rx2 = sender.subscribe();
    }
}
