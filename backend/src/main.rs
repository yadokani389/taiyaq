use std::net::SocketAddr;

use api::route::routes;
use dotenvy::dotenv;
use poise::serenity_prelude::*;

use crate::app::AppRegistry;

mod api;
mod app;
mod data;
pub mod discord;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let line_token =
        std::env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("Missing LINE_CHANNEL_ACCESS_TOKEN");

    let framework = discord::framework_builder()
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let registry = AppRegistry::new(line_token, ctx.clone());
                registry.load_data().await.ok();

                let app = routes().with_state(registry.clone());

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
