use axum::{extract::State, http::StatusCode, Json};
use bot_sdk_line::webhook_line::models::CallbackRequest;

use crate::app::AppRegistry;
use crate::line::handler;

pub async fn line_callback(
    State(registry): State<AppRegistry>,
    Json(req): Json<CallbackRequest>,
) -> StatusCode {
    println!("req: {req:#?}");

    match handler::line_handler(&registry, req).await {
        Ok(_) => StatusCode::OK,
        Err(status) => status,
    }
}