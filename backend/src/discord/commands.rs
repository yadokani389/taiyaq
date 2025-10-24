use poise::serenity_prelude::*;

use crate::{
    api::model::AddNotificationRequest,
    data::{NotifyChannel, OrderStatus},
};

use super::PoiseContext;

/// ユーザー向け注文コマンド
#[poise::command(slash_command, subcommands("display", "details", "notify"))]
pub async fn orders(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    ctx.say("サブコマンドを使用してください。例: `/orders display`")
        .await?;
    Ok(())
}

/// 公開ディスプレイ画面用の注文を取得します
#[poise::command(slash_command)]
async fn display(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    let orders = &ctx.data().data().await.orders;
    let ready: Vec<_> = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Ready)
        .map(|o| o.id.to_string())
        .collect();
    let cooking: Vec<_> = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Cooking)
        .map(|o| o.id.to_string())
        .collect();
    let waiting: Vec<_> = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Waiting)
        .map(|o| o.id.to_string())
        .collect();

    let embed = CreateEmbed::default()
        .title("注文状況表示")
        .field(
            "受け取り準備完了",
            if ready.is_empty() {
                "なし".to_string()
            } else {
                ready.join(" | ")
            },
            false,
        )
        .field(
            "調理中",
            if cooking.is_empty() {
                "なし".to_string()
            } else {
                cooking.join(" | ")
            },
            false,
        )
        .field(
            "待機中",
            if waiting.is_empty() {
                "なし".to_string()
            } else {
                waiting.join(" | ")
            },
            false,
        )
        .timestamp(Timestamp::now());

    let builder = poise::CreateReply::default().embed(embed);
    ctx.send(builder).await?;

    Ok(())
}

/// 特定の注文の詳細を取得します
#[poise::command(slash_command)]
async fn details(
    ctx: PoiseContext<'_>,
    #[description = "あなたの注文ID"] id: u32,
) -> Result<(), anyhow::Error> {
    if let Some(details) = ctx.data().get_order_details(id).await {
        let response = format!(
            "注文 `{}`: ステータスは `{:?}` です。推定待ち時間: `{}`.",
            details.id,
            details.status,
            details
                .estimated_wait_minutes
                .map_or("N/A".to_string(), |m| format!("{} 分", m))
        );
        ctx.say(response).await?;
    } else {
        ctx.say(format!("注文 `{}` が見つかりません。", id)).await?;
    }
    Ok(())
}

/// 注文に通知を追加します
#[poise::command(slash_command)]
async fn notify(
    ctx: PoiseContext<'_>,
    #[description = "注文ID"] id: u32,
) -> Result<(), anyhow::Error> {
    let registry = ctx.data();
    let data = registry.data().await;
    let order = data.orders.iter().find(|o| o.id == id);

    if order.is_none() {
        let builder =
            poise::CreateReply::default().content(format!("注文 `{}` が見つかりません。", id));
        ctx.send(builder).await?;
        return Ok(());
    }

    let order = order.unwrap().clone();
    drop(data);

    if order.status == OrderStatus::Completed || order.status == OrderStatus::Cancelled {
        let builder = poise::CreateReply::default().content(format!(
            "注文 `{}` はすでに完了/キャンセルされています。",
            id
        ));
        ctx.send(builder).await?;
        return Ok(());
    }

    let items_str = order
        .items
        .iter()
        .map(|item| format!("- {} x{}", item.flavor, item.quantity))
        .collect::<Vec<_>>()
        .join(
            "
",
        );

    let ordered_at_str = format!("<t:{}:F>", order.ordered_at.timestamp());

    let embed = CreateEmbed::default()
        .title(format!("注文 #{} の通知設定", order.id))
        .description("以下の注文で通知を登録しますか？")
        .field("商品", items_str.clone(), false)
        .field("注文時刻", ordered_at_str.clone(), false)
        .color(Colour::ORANGE);

    let custom_id_confirm = format!("notify_confirm_{}_{}", id, ctx.id());
    let custom_id_cancel = format!("notify_cancel_{}_{}", id, ctx.id());

    let builder =
        poise::CreateReply::default()
            .embed(embed)
            .components(vec![CreateActionRow::Buttons(vec![
                CreateButton::new(custom_id_confirm.clone())
                    .label("はい、登録する")
                    .style(ButtonStyle::Success),
                CreateButton::new(custom_id_cancel.clone())
                    .label("いいえ")
                    .style(ButtonStyle::Danger),
            ])]);

    let reply_handle = ctx.send(builder).await?;

    let interaction = {
        if let Ok(message) = reply_handle.message().await {
            message
                .await_component_interaction(ctx)
                .author_id(ctx.author().id)
                .timeout(std::time::Duration::from_secs(60))
                .await
        } else {
            None
        }
    };

    let mut edited_embed = CreateEmbed::default()
        .title(format!("注文 #{} の通知設定", order.id))
        .field("商品", items_str, false)
        .field("注文時刻", ordered_at_str, false);

    match interaction {
        Some(press) => {
            press.defer(ctx).await?;
            if press.data.custom_id == custom_id_confirm {
                let payload = AddNotificationRequest {
                    channel: NotifyChannel::Discord,
                    target: ctx.author().id.to_string(),
                };
                if registry.add_notification(id, payload).await.is_some() {
                    edited_embed = edited_embed
                        .description("通知を登録しました。準備ができたらDMでお知らせします。")
                        .color(Colour::DARK_GREEN);
                } else {
                    edited_embed = edited_embed
                        .description("エラー：通知の登録に失敗しました。")
                        .color(Colour::RED);
                }
            } else {
                edited_embed = edited_embed.description("通知の登録をキャンセルしました。");
            }
        }
        None => {
            edited_embed = edited_embed
                .description("タイムアウトしました。再度コマンドを実行してください。")
                .color(Colour::RED);
        }
    }

    let builder = poise::CreateReply::default()
        .embed(edited_embed)
        .components(vec![]);
    reply_handle.edit(ctx, builder).await?;

    Ok(())
}
