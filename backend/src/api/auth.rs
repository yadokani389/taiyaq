use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::env;

const AUTH_HEADER_NAME: &str = "Authorization";

pub async fn staff_api_auth(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(AUTH_HEADER_NAME)
        .and_then(|header| header.to_str().ok());

    let api_token = env::var("STAFF_API_TOKEN").expect("STAFF_API_TOKEN must be set");

    if let Some(auth_header) = auth_header
        && let Some(token) = auth_header.strip_prefix("Bearer ")
        && token == api_token
    {
        return Ok(next.run(req).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}
