use poise::FrameworkBuilder;
use poise::serenity_prelude::*;

use crate::app::AppRegistry;

mod commands;

pub type PoiseContext<'a> = poise::Context<'a, AppRegistry, anyhow::Error>;

pub fn framework_builder() -> FrameworkBuilder<AppRegistry, anyhow::Error> {
    poise::Framework::builder().options(poise::FrameworkOptions {
        commands: vec![commands::orders()],
        ..Default::default()
    })
}

pub async fn send_dm(ctx: &Context, user_id: u64, message: &str) -> anyhow::Result<()> {
    let builder = CreateMessage::new().content(message);
    UserId::from(user_id).direct_message(ctx, builder).await?;
    Ok(())
}
