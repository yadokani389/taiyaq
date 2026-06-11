use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::app::AppRegistry;

const AUTH_HEADER_NAME: &str = "Authorization";

pub async fn staff_api_auth(
    State(registry): State<AppRegistry>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(AUTH_HEADER_NAME)
        .and_then(|header| header.to_str().ok());

    if let Some(auth_header) = auth_header
        && let Some(token) = auth_header.strip_prefix("Bearer ")
        && token == registry.staff_api_token()
    {
        return Ok(next.run(req).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}
