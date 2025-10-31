use poise::Command;
use poise::FrameworkBuilder;
use poise::serenity_prelude::*;

use crate::app::AppRegistry;

mod commands;
mod event_handler;

pub type PoiseContext<'a> = poise::Context<'a, AppRegistry, anyhow::Error>;

const CREATE_CHANNEL: &str = "create_channel";

pub fn framework_builder() -> FrameworkBuilder<AppRegistry, anyhow::Error> {
    let mut commands = global_commands();
    commands.append(&mut guild_commands());
    poise::Framework::builder().options(poise::FrameworkOptions {
        commands,
        require_cache_for_guild_check: false,
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler::event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    })
}

pub fn global_commands() -> Vec<Command<AppRegistry, anyhow::Error>> {
    vec![
        commands::orders(),
        commands::waittime(),
        commands::create_channel_button(),
    ]
}

pub fn guild_commands() -> Vec<Command<AppRegistry, anyhow::Error>> {
    vec![commands::staff()]
}

pub async fn send_notification(
    ctx: &Context,
    channel_id: u64,
    user_id: u64,
    message: &str,
) -> anyhow::Result<()> {
    let builder = CreateMessage::new().content(format!("<@{user_id}>\n{message}"));
    ChannelId::from(channel_id)
        .send_message(ctx, builder)
        .await?;
    Ok(())
}
