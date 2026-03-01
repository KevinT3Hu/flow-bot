//! Authentication module for web interface

use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::web::WebState;

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub message: String,
}

/// Auth error
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    WrongCredentials,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid authentication token"),
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Invalid password"),
        };

        let body = serde_json::json!({ "error": message });
        (status, Json(body)).into_response()
    }
}

/// Generate a JWT token
pub fn generate_token(secret: &str) -> anyhow::Result<String> {
    let now = Utc::now();
    let exp = now + Duration::hours(24);

    let claims = Claims {
        sub: "admin".to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &encoding_key).map_err(|e| anyhow::anyhow!(e))
}

/// Validate a JWT token
pub fn validate_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);

    let decoded = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(decoded.claims)
}

/// Verify password
pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    bcrypt::verify(password, hash).map_err(|e| anyhow::anyhow!(e))
}

/// Hash password
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| anyhow::anyhow!(e))
}

/// Login handler
pub async fn login(
    State(state): State<Arc<WebState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // If no password is set, allow any password
    let valid = if let Some(ref hash) = state.password_hash {
        verify_password(&req.password, hash).unwrap_or(false)
    } else {
        true
    };

    if valid {
        match generate_token(&state.jwt_secret) {
            Ok(token) => {
                let response = LoginResponse {
                    success: true,
                    token: Some(token),
                    message: "Login successful".to_string(),
                };
                (StatusCode::OK, Json(response))
            }
            Err(e) => {
                let response = LoginResponse {
                    success: false,
                    token: None,
                    message: format!("Failed to generate token: {}", e),
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
    } else {
        let response = LoginResponse {
            success: false,
            token: None,
            message: "Invalid password".to_string(),
        };
        (StatusCode::UNAUTHORIZED, Json(response))
    }
}

/// Auth middleware
pub async fn auth_middleware(
    State(state): State<Arc<WebState>>,
    req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Skip auth if no password is set
    if state.password_hash.is_none() {
        return Ok(next.run(req).await);
    }

    // Extract token from header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;

    // Validate token
    validate_token(token, &state.jwt_secret).map_err(|_| AuthError::InvalidToken)?;

    Ok(next.run(req).await)
}

/// Generate a random JWT secret
pub fn generate_jwt_secret() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}
