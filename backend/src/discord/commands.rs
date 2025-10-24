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
    let target_user = ctx.author().id;
    let payload = AddNotificationRequest {
        channel: NotifyChannel::Discord,
        target: target_user.to_string(),
    };

    // Check if the order exists and is in a valid state for notifications
    let order_exists = ctx.data().data().await.orders.iter().any(|o| {
        o.id == id && o.status != OrderStatus::Completed && o.status != OrderStatus::Cancelled
    });

    if !order_exists {
        ctx.say(format!(
            "注文 `{}` が見つからないか、すでに完了/キャンセルされています。",
            id
        ))
        .await?;
        return Ok(());
    }

    if let Some(order) = ctx.data().add_notification(id, payload).await {
        ctx.say(format!(
            "注文 `{}` の通知が設定されました。準備ができた際にお知らせします。",
            order.id
        ))
        .await?;
    } else {
        ctx.say(format!("注文 `{}` の通知設定に失敗しました。", id))
            .await?;
    }
    Ok(())
}
