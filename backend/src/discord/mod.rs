use poise::Command;
use poise::FrameworkBuilder;
use poise::serenity_prelude::*;

use crate::app::AppRegistry;

mod commands;

pub type PoiseContext<'a> = poise::Context<'a, AppRegistry, anyhow::Error>;

pub fn framework_builder() -> FrameworkBuilder<AppRegistry, anyhow::Error> {
    let mut commands = global_commands();
    commands.append(&mut guild_commands());
    poise::Framework::builder().options(poise::FrameworkOptions {
        commands,
        require_cache_for_guild_check: false,
        ..Default::default()
    })
}

pub fn global_commands() -> Vec<Command<AppRegistry, anyhow::Error>> {
    vec![commands::orders(), commands::waittime()]
}

pub fn guild_commands() -> Vec<Command<AppRegistry, anyhow::Error>> {
    vec![commands::staff()]
}

pub async fn send_dm(ctx: &Context, user_id: u64, message: &str) -> anyhow::Result<()> {
    let builder = CreateMessage::new().content(message);
    UserId::from(user_id).direct_message(ctx, builder).await?;
    Ok(())
}
