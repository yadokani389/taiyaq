use axum::{body::Bytes, extract::State, http::StatusCode};
use bot_sdk_line::{parser::signature::validate_signature, support::axum::Signature};

use crate::app::AppRegistry;
use crate::line::handler;

pub async fn line_callback(
    State(registry): State<AppRegistry>,
    Signature(signature): Signature,
    body: Bytes,
) -> StatusCode {
    let Ok(channel_secret) = std::env::var("LINE_CHANNEL_SECRET") else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    let Ok(body_str) = std::str::from_utf8(&body) else {
        return StatusCode::BAD_REQUEST;
    };

    if !validate_signature(&channel_secret, &signature, body_str) {
        return StatusCode::UNAUTHORIZED;
    }

    let Ok(req) = serde_json::from_slice(&body) else {
        return StatusCode::BAD_REQUEST;
    };

    match handler::line_handler(&registry, req).await {
        Ok(_) => StatusCode::OK,
        Err(status) => status,
    }
}
