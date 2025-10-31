use axum::{Json, extract::State, http::StatusCode};
use bot_sdk_line::webhook_line::models::CallbackRequest;

use crate::app::AppRegistry;
use crate::line::handler;

pub async fn line_callback(
    State(registry): State<AppRegistry>,
    Json(req): Json<CallbackRequest>,
) -> StatusCode {
    match handler::line_handler(&registry, req).await {
        Ok(_) => StatusCode::OK,
        Err(status) => status,
    }
}
