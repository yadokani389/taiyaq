use axum::http::{HeaderName, Method};
use dotenvy::dotenv;
use poise::serenity_prelude::*;
use taiyaq_backend::api::route::routes;
use taiyaq_backend::app::AppRegistry;
use taiyaq_backend::config::Config;
use taiyaq_backend::discord;
use taiyaq_backend::storage::{self, SqliteRepository};
use tower_http::cors::{self, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;
    let pool = storage::connect(&config.database_url).await?;
    let repository = SqliteRepository::new(pool);
    let setup_config = config.clone();
    let discord_token = config.discord_token.clone();

    let framework = discord::framework_builder()
        .setup(move |ctx, _ready, _framework| {
            let config = setup_config.clone();
            let repository = repository.clone();
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &discord::global_commands()).await?;
                poise::builtins::register_in_guild(
                    ctx,
                    &discord::guild_commands(),
                    config.discord_guild_id.into(),
                )
                .await?;

                let registry = AppRegistry::new(
                    config.line_channel_access_token,
                    config.line_channel_secret,
                    config.staff_api_token,
                    ctx.clone(),
                    repository,
                );
                let ret = registry.initialize().await;
                info!(?ret, "initialized registry");

                let app = routes(registry.clone()).layer(cors());

                info!(addr = %config.bind_addr, "listening");
                let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

                tokio::spawn(async move {
                    axum::serve(listener, app)
                        .await
                        .expect("Failed to start API server");
                });

                Ok(registry)
            })
        })
        .build();

    let intents = GatewayIntents::non_privileged();

    let client = ClientBuilder::new(discord_token, intents)
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
