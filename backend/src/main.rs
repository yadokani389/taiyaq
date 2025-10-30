use std::net::SocketAddr;

use api::route::routes;
use axum::http::{HeaderName, Method};
use dotenvy::dotenv;
use poise::serenity_prelude::*;
use tower_http::cors::{self, CorsLayer};

use crate::app::AppRegistry;

mod api;
mod app;
mod data;
pub mod discord;
pub mod line;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let line_token =
        std::env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("Missing LINE_CHANNEL_ACCESS_TOKEN");
    std::env::var("STAFF_API_TOKEN").expect("Missing STAFF_API_TOKEN");

    let framework = discord::framework_builder()
        .setup(|ctx, _ready, _framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &discord::global_commands()).await?;
                let guild_id: u64 = std::env::var("DISCORD_GUILD_ID")
                    .expect("Missing DISCORD_GUILD_ID")
                    .parse()
                    .expect("DISCORD_GUILD_ID must be a valid u64");
                poise::builtins::register_in_guild(
                    ctx,
                    &discord::guild_commands(),
                    guild_id.into(),
                )
                .await?;

                let registry = AppRegistry::new(line_token, ctx.clone());
                let ret = registry.load_data().await;
                println!("Load data result: {:?}", ret);

                let app = routes().with_state(registry.clone()).layer(cors());

                let addr = SocketAddr::from(([127, 0, 0, 1], 38000));
                println!("listening on {}", addr);
                let listener = tokio::net::TcpListener::bind(addr).await?;

                tokio::spawn(async move {
                    axum::serve(listener, app)
                        .await
                        .expect("Failed to start API server");
                });

                Ok(registry)
            })
        })
        .build();

    let token = std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");
    let intents = GatewayIntents::non_privileged();

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client
        .expect("Failed to create client")
        .start()
        .await
        .expect("Failed to start client");

    Ok(())
}

fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_headers(vec![
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
            HeaderName::from_static("accept"),
        ])
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_origin(cors::Any)
}
