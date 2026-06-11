use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub line_channel_access_token: String,
    pub line_channel_secret: String,
    pub staff_api_token: String,
    pub discord_token: String,
    pub discord_guild_id: u64,
    pub bind_addr: SocketAddr,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/taiyaq.sqlite".to_string()),
            line_channel_access_token: required_var("LINE_CHANNEL_ACCESS_TOKEN")?,
            line_channel_secret: required_var("LINE_CHANNEL_SECRET")?,
            staff_api_token: required_var("STAFF_API_TOKEN")?,
            discord_token: required_var("DISCORD_TOKEN")?,
            discord_guild_id: required_var("DISCORD_GUILD_ID")?.parse().map_err(|error| {
                anyhow::anyhow!("DISCORD_GUILD_ID must be a valid u64: {error}")
            })?,
            bind_addr: std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:38000".to_string())
                .parse()
                .map_err(|error| {
                    anyhow::anyhow!("BIND_ADDR must be a valid socket address: {error}")
                })?,
        })
    }
}

fn required_var(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|_| anyhow::anyhow!("Missing {name}"))
}
