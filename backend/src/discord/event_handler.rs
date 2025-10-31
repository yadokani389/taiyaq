use poise::serenity_prelude::*;

use crate::{app::AppRegistry, discord::CREATE_CHANNEL};

pub async fn event_handler(
    ctx: &Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, AppRegistry, anyhow::Error>,
    _registry: &AppRegistry,
) -> Result<(), anyhow::Error> {
    if let FullEvent::InteractionCreate { interaction } = event
        && let Interaction::Component(interaction) = interaction
        && interaction.data.custom_id.starts_with(CREATE_CHANNEL)
        && let Some(guild_id) = interaction.guild_id
        && let Some(category_id) = interaction
            .data
            .custom_id
            .split('_')
            .next_back()
            .and_then(|last| last.parse::<u64>().ok())
    {
        let user_id = interaction.user.id;
        if guild_id
            .channels(ctx)
            .await?
            .iter()
            .any(|(_, guild)| guild.name == user_id.to_string())
        {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("すでにチャンネルがあります")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
        let builder = CreateChannel::new(user_id.to_string())
            .permissions(vec![PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user_id),
            }])
            .category(category_id);
        guild_id.create_channel(ctx, builder).await?;
        interaction
            .create_response(ctx, CreateInteractionResponse::Acknowledge)
            .await?;
    }
    Ok(())
}
