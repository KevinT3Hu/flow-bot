//! Web management interface for Flow-Bot
//!
//! This module provides a web-based UI for managing the bot, including:
//! - Real-time log viewing via WebSocket
//! - Plugin management (enable/disable)
//! - Bot status and information
//! - JWT-based authentication

pub mod auth;
pub mod log_collector;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context as AnyhowContext, Result};
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket};
use axum::http::{Response, header};
use axum::middleware;
use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;

use crate::runtime::PluginManager;
use auth::{auth_middleware, login};
pub use log_collector::LogMessage;

#[cfg(feature = "webui")]
use rust_embed::RustEmbed;

#[cfg(feature = "webui")]
/// Embedded static files from the web UI build
#[derive(RustEmbed)]
#[folder = "src/web/static/"]
struct StaticAssets;

/// Shared state between web handlers and the bot runtime
pub struct WebState {
    /// Access to the plugin manager for enable/disable operations
    plugin_manager: Arc<PluginManager>,
    /// Broadcast channel for log messages
    log_tx: broadcast::Sender<LogMessage>,
    /// Bot start time for uptime calculation
    start_time: Instant,
    /// Connection mode string
    connection_mode: String,
    /// Password hash for authentication (None = no auth required)
    password_hash: Option<String>,
    /// JWT secret for token signing
    jwt_secret: String,
}

/// Bot information response
#[derive(Debug, Serialize)]
struct BotInfo {
    version: String,
    uptime_seconds: u64,
    connection_mode: String,
    plugin_count: usize,
    total_plugins_in_dir: usize,
    auth_enabled: bool,
}

/// Plugin status response
#[derive(Debug, Serialize)]
struct PluginStatus {
    name: String,
    version: String,
    description: String,
    enabled: bool,
    loaded_at: Option<String>,
}

/// Success response
#[derive(Debug, Serialize)]
struct SuccessResponse {
    success: bool,
    message: String,
}

/// Error response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// Web error type
#[derive(Debug)]
enum WebError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for WebError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            WebError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            WebError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse { error: message });
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for WebError {
    fn from(err: anyhow::Error) -> Self {
        WebError::Internal(err.to_string())
    }
}

/// Start the web server on the specified bind address
pub async fn start_web_server(
    bind_addr: &str,
    plugin_manager: Arc<PluginManager>,
    log_tx: broadcast::Sender<LogMessage>,
    connection_mode: String,
    password: Option<String>,
) -> Result<()> {
    let password_hash = match password {
        Some(p) if !p.is_empty() => Some(auth::hash_password(&p)?),
        _ => None,
    };

    let jwt_secret = auth::generate_jwt_secret();

    if password_hash.is_some() {
        tracing::info!("Web interface authentication enabled");
    } else {
        tracing::warn!("Web interface authentication disabled - no password set");
    }

    let state = Arc::new(WebState {
        plugin_manager,
        log_tx,
        start_time: Instant::now(),
        connection_mode,
        password_hash,
        jwt_secret,
    });

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/api/login", post(login))
        .route("/api/info", get(get_info));

    // Protected API routes (auth required if password is set)
    let protected_routes = Router::new()
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/:name/enable", post(enable_plugin))
        .route("/api/plugins/:name/disable", post(disable_plugin))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // WebSocket route (auth required if password is set)
    let ws_routes =
        Router::new()
            .route("/ws/logs", get(logs_websocket))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware_ws,
            ));

    // Build router
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_routes)
        // Static files - serve the embedded web UI
        .fallback(get(serve_static))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("Failed to parse bind address: {}", bind_addr))?;

    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind web server to {}", addr))?;

    tracing::info!("Web management interface started on http://{}", addr);

    axum::serve(listener, app)
        .await
        .context("Web server error")?;

    Ok(())
}

/// Auth middleware for WebSocket connections
async fn auth_middleware_ws(
    State(state): State<Arc<WebState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<impl IntoResponse, auth::AuthError> {
    // Skip auth if no password is set
    if state.password_hash.is_none() {
        return Ok(next.run(req).await);
    }

    // For WebSocket, check token in query parameter
    let uri = req.uri();
    let query = uri.query().unwrap_or("");
    let params: std::collections::HashMap<_, _> =
        url::form_urlencoded::parse(query.as_bytes()).collect();

    if let Some(token) = params.get("token") {
        auth::validate_token(token, &state.jwt_secret)
            .map_err(|_| auth::AuthError::InvalidToken)?;
        Ok(next.run(req).await)
    } else {
        // Also check Authorization header
        let auth_header = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        if let Some(header) = auth_header {
            let token = header
                .strip_prefix("Bearer ")
                .ok_or(auth::AuthError::InvalidToken)?;
            auth::validate_token(token, &state.jwt_secret)
                .map_err(|_| auth::AuthError::InvalidToken)?;
            Ok(next.run(req).await)
        } else {
            Err(auth::AuthError::MissingToken)
        }
    }
}

/// GET /api/info - Return basic bot info (version, uptime, plugin count)
async fn get_info(State(state): State<Arc<WebState>>) -> Json<BotInfo> {
    let plugin_count = state.plugin_manager.plugin_count().await;
    let total_plugins = state.plugin_manager.scan_available_plugins().await.len();

    Json(BotInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        connection_mode: state.connection_mode.clone(),
        plugin_count,
        total_plugins_in_dir: total_plugins,
        auth_enabled: state.password_hash.is_some(),
    })
}

/// GET /api/plugins - List all plugins with their status
async fn list_plugins(State(state): State<Arc<WebState>>) -> Json<Vec<PluginStatus>> {
    let available_plugins = state.plugin_manager.scan_available_plugins().await;
    let loaded_plugins = state.plugin_manager.get_plugin_info().await;

    let mut statuses = Vec::new();

    // Build status for each available plugin
    for path in available_plugins {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Check if this plugin is loaded
        let loaded_info = loaded_plugins.iter().find(|p| p.name == name);

        statuses.push(PluginStatus {
            name: name.clone(),
            version: loaded_info.map(|p| p.version.clone()).unwrap_or_default(),
            description: loaded_info
                .map(|p| p.description.clone())
                .unwrap_or_default(),
            enabled: loaded_info.is_some(),
            loaded_at: loaded_info.map(|p| p.loaded_at.to_rfc3339()),
        });
    }

    // Also include any loaded plugins that might not be in the scan (edge case)
    for plugin in &loaded_plugins {
        if !statuses.iter().any(|s| s.name == plugin.name) {
            statuses.push(PluginStatus {
                name: plugin.name.clone(),
                version: plugin.version.clone(),
                description: plugin.description.clone(),
                enabled: true,
                loaded_at: Some(plugin.loaded_at.to_rfc3339()),
            });
        }
    }

    statuses.sort_by(|a, b| a.name.cmp(&b.name));
    Json(statuses)
}

/// POST /api/plugins/{name}/enable - Enable a plugin (load it)
async fn enable_plugin(
    Path(name): Path<String>,
    State(state): State<Arc<WebState>>,
) -> Result<Json<SuccessResponse>, WebError> {
    // Check if plugin is already loaded
    if state.plugin_manager.is_plugin_loaded(&name).await {
        return Ok(Json(SuccessResponse {
            success: true,
            message: format!("Plugin '{}' is already enabled", name),
        }));
    }

    // Try to enable the plugin
    state
        .plugin_manager
        .enable_plugin(&name)
        .await
        .map_err(|e| WebError::Internal(e.to_string()))?;

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Plugin '{}' enabled successfully", name),
    }))
}

/// POST /api/plugins/{name}/disable - Disable a plugin (unload it)
async fn disable_plugin(
    Path(name): Path<String>,
    State(state): State<Arc<WebState>>,
) -> Result<Json<SuccessResponse>, WebError> {
    // Check if plugin is loaded
    if !state.plugin_manager.is_plugin_loaded(&name).await {
        return Err(WebError::NotFound(format!(
            "Plugin '{}' is not enabled",
            name
        )));
    }

    // Try to disable the plugin
    state
        .plugin_manager
        .unload_plugin(&name)
        .await
        .map_err(|e| WebError::Internal(e.to_string()))?;

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Plugin '{}' disabled successfully", name),
    }))
}

/// WebSocket /ws/logs - Stream realtime logs to connected clients
async fn logs_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection for log streaming
async fn handle_socket(socket: WebSocket, state: Arc<WebState>) {
    let rx = state.log_tx.subscribe();

    let (mut sender, _receiver) = socket.split();

    // Create a stream from the broadcast receiver
    let mut stream = BroadcastStream::new(rx.resubscribe());

    while let Some(Ok(log_msg)) = stream.next().await {
        let json = match serde_json::to_string(&log_msg) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Failed to serialize log message: {}", e);
                continue;
            }
        };

        if sender.send(Message::Text(json.into())).await.is_err() {
            // Client disconnected
            break;
        }
    }
}

/// Serve embedded static files or simple message if webui is not enabled
async fn serve_static(_request: axum::extract::Request) -> impl IntoResponse {
    #[cfg(feature = "webui")]
    {
        let path = _request.uri().path().trim_start_matches('/');

        // Try to get the requested file
        let file = if path.is_empty() {
            StaticAssets::get("index.html")
        } else {
            StaticAssets::get(path).or_else(|| {
                // Try with .html extension for SPA routes
                StaticAssets::get(&format!("{}.html", path))
            })
        };

        if let Some(file) = file {
            let content_type = get_content_type(path);
            Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(file.data))
                .unwrap()
        } else {
            // Serve index.html for SPA routing (fallback)
            if let Some(index) = StaticAssets::get("index.html") {
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(Body::from(index.data))
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not found"))
                    .unwrap()
            }
        }
    }

    #[cfg(not(feature = "webui"))]
    {
        // API-only mode - return simple message
        Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"message": "Flow-Bot API Server - Web UI not enabled. Build with --features webui to include the web interface."}"#))
            .unwrap()
    }
}

#[cfg(feature = "webui")]
/// Get content type based on file extension
fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".html") || path.is_empty() {
        "text/html"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else if path.ends_with(".ttf") {
        "font/ttf"
    } else {
        "application/octet-stream"
    }
}
