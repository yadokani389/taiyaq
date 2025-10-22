use std::{env, net::SocketAddr};

use api::route::routes;
use dotenvy::dotenv;

use crate::app::AppRegistry;

mod api;
mod app;
mod data;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let line_token: String =
        env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("Failed getting LINE_CHANNEL_ACCESS_TOKEN");

    let registry = AppRegistry::from_file(line_token).await?;

    let app = routes().with_state(registry.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], 38000));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    registry.save_data().await?;

    Ok(())
}
