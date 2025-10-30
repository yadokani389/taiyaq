use poise::serenity_prelude::*;

use crate::{
    api::model::AddNotificationRequest,
    data::{Flavor, FlavorConfig, Item, NotifyChannel, OrderStatus},
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
        .join("\n");

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
            let (description, color) = if press.data.custom_id == custom_id_confirm {
                let payload = AddNotificationRequest {
                    channel: NotifyChannel::Discord,
                    target: ctx.author().id.to_string(),
                };
                if registry.add_notification(id, payload).await.is_some() {
                    (
                        "通知を登録しました。準備ができたらDMでお知らせします。",
                        Colour::DARK_GREEN,
                    )
                } else {
                    ("エラー：通知の登録に失敗しました。", Colour::RED)
                }
            } else {
                ("通知の登録をキャンセルしました。", Colour::default())
            };
            edited_embed = edited_embed.description(description).color(color);
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

/// 現在の待ち時間を表示します
#[poise::command(slash_command)]
pub async fn waittime(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    let wait_times = ctx.data().get_current_wait_times().await;
    let mut fields = Vec::new();

    for (flavor, time) in wait_times.wait_times {
        let time_str = time.map_or("提供なし".into(), |t| {
            if t == 0 {
                "すぐに提供できます".into()
            } else {
                format!("約{}分", t)
            }
        });
        fields.push((flavor.to_string(), time_str, false));
    }

    let embed = CreateEmbed::default()
        .title("現在の待ち時間")
        .fields(fields)
        .timestamp(Timestamp::now());

    let builder = poise::CreateReply::default().embed(embed);
    ctx.send(builder).await?;

    Ok(())
}

// Helper function for parsing flavor from string
fn parse_flavor(s: &str) -> Result<Flavor, &'static str> {
    match s.to_lowercase().as_str() {
        "tsubuan" => Ok(Flavor::Tsubuan),
        "custard" => Ok(Flavor::Custard),
        "kurikinton" => Ok(Flavor::Kurikinton),
        _ => Err("不正なフレーバーです"),
    }
}

// Helper choice enum for flavor
#[derive(Debug, Clone, poise::ChoiceParameter)]
pub enum FlavorChoice {
    Tsubuan,
    Custard,
    Kurikinton,
}

impl From<FlavorChoice> for Flavor {
    fn from(choice: FlavorChoice) -> Self {
        match choice {
            FlavorChoice::Tsubuan => Flavor::Tsubuan,
            FlavorChoice::Custard => Flavor::Custard,
            FlavorChoice::Kurikinton => Flavor::Kurikinton,
        }
    }
}

/// スタッフ向け管理コマンド
#[poise::command(
    slash_command,
    subcommands(
        "get_orders",
        "create_order",
        "update_production",
        "complete_order",
        "cancel_order",
        "update_order_priority",
        "get_flavor_configs",
        "set_flavor_config"
    ),
    guild_only
)]
pub async fn staff(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    ctx.say("サブコマンドを使用してください。例: `/staff get_orders`")
        .await?;
    Ok(())
}

/// 注文一覧を取得します
#[poise::command(slash_command, rename = "get_orders")]
async fn get_orders(
    ctx: PoiseContext<'_>,
    #[description = "ステータスで絞り込み (カンマ区切り: waiting,cooking,ready,completed,cancelled)"]
    status: Option<String>,
) -> Result<(), anyhow::Error> {
    let data = ctx.data().data().await;
    let orders = &data.orders;

    let statuses: Vec<OrderStatus> = if let Some(s) = status {
        s.split(',')
            .filter_map(|s_trim| {
                let s = s_trim.trim();
                match s {
                    "waiting" => Some(OrderStatus::Waiting),
                    "cooking" => Some(OrderStatus::Cooking),
                    "ready" => Some(OrderStatus::Ready),
                    "completed" => Some(OrderStatus::Completed),
                    "cancelled" => Some(OrderStatus::Cancelled),
                    _ => None,
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let filtered_orders: Vec<_> = if statuses.is_empty() {
        orders.to_vec()
    } else {
        orders
            .iter()
            .filter(|o| statuses.contains(&o.status))
            .cloned()
            .collect()
    };

    let mut response = String::new();
    for order in filtered_orders.iter().take(10) {
        let items_str = order
            .items
            .iter()
            .map(|i| format!("{} x {}", i.flavor, i.quantity))
            .collect::<Vec<_>>()
            .join(", ");
        response.push_str(&format!(
            "ID: `{}` | Status: `{:?}` | Priority: `{}` | Items: `{}`\n",
            order.id, order.status, order.is_priority, items_str
        ));
    }

    if filtered_orders.len() > 10 {
        response.push_str(&format!(
            "\n... and {} more orders.",
            filtered_orders.len() - 10
        ));
    }

    if response.is_empty() {
        response = "対象の注文はありません。".to_string();
    }

    ctx.say(response).await?;

    Ok(())
}

/// 新しい注文を作成します
#[poise::command(slash_command)]
async fn create_order(
    ctx: PoiseContext<'_>,
    #[description = "注文アイテム (例: tsubuan:2,custard:1,kurikinton:1)"] items: String,
    #[description = "優先注文にするか"] is_priority: Option<bool>,
) -> Result<(), anyhow::Error> {
    let mut parsed_items = Vec::<Item>::new();
    for item_str in items.split(',') {
        let parts: Vec<&str> = item_str.trim().split(':').collect();
        if parts.len() != 2 {
            ctx.say("アイテムのフォーマットが不正です。例: `tsubuan:2,custard:1,kurikinton:1`")
                .await?;
            return Ok(());
        }
        let flavor = match parse_flavor(parts[0]) {
            Ok(f) => f,
            Err(e) => {
                ctx.say(format!("{}: `{}`", e, parts[0])).await?;
                return Ok(());
            }
        };
        let quantity = match parts[1].parse::<usize>() {
            Ok(q) => q,
            Err(_) => {
                ctx.say(format!("不正な数量です: `{}`", parts[1])).await?;
                return Ok(());
            }
        };
        parsed_items.push(Item { flavor, quantity });
    }

    if parsed_items.is_empty() {
        ctx.say("アイテムが指定されていません。").await?;
        return Ok(());
    }

    let new_order = ctx
        .data()
        .create_order(parsed_items, is_priority.unwrap_or(false))
        .await;

    ctx.say(format!("新しい注文を作成しました。ID: {}", new_order.id))
        .await?;

    Ok(())
}

/// 生産完了を報告します
#[poise::command(slash_command)]
async fn update_production(
    ctx: PoiseContext<'_>,
    #[description = "生産したアイテム (例: tsubuan:9,custard:9)"] items: String,
) -> Result<(), anyhow::Error> {
    let mut parsed_items = Vec::<Item>::new();
    for item_str in items.split(',') {
        let parts: Vec<&str> = item_str.trim().split(':').collect();
        if parts.len() != 2 {
            ctx.say("アイテムのフォーマットが不正です。例: `tsubuan:9,custard:9`")
                .await?;
            return Ok(());
        }
        let flavor = match parse_flavor(parts[0]) {
            Ok(f) => f,
            Err(e) => {
                ctx.say(format!("{}: `{}`", e, parts[0])).await?;
                return Ok(());
            }
        };
        let quantity = match parts[1].parse::<usize>() {
            Ok(q) => q,
            Err(_) => {
                ctx.say(format!("不正な数量です: `{}`", parts[1])).await?;
                return Ok(());
            }
        };
        parsed_items.push(Item { flavor, quantity });
    }

    if parsed_items.is_empty() {
        ctx.say("アイテムが指定されていません。").await?;
        return Ok(());
    }

    let (newly_ready_orders, unallocated_items) = ctx.data().update_production(parsed_items).await;

    let ready_str = if newly_ready_orders.is_empty() {
        "なし".to_string()
    } else {
        newly_ready_orders
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let unallocated_str = if unallocated_items.is_empty() {
        "なし".to_string()
    } else {
        unallocated_items
            .iter()
            .map(|item| format!("{} x {}", item.flavor, item.quantity))
            .collect::<Vec<_>>()
            .join(", ")
    };

    ctx.say(format!(
        "生産を報告しました。\n新たに準備完了になった注文: {}\n余剰在庫: {}",
        ready_str, unallocated_str
    ))
    .await?;

    Ok(())
}

/// 注文を完了にします
#[poise::command(slash_command)]
async fn complete_order(
    ctx: PoiseContext<'_>,
    #[description = "注文ID"] id: u32,
) -> Result<(), anyhow::Error> {
    if let Some(order) = ctx.data().complete_order(id).await {
        ctx.say(format!("注文 `{}` を完了にしました。", order.id))
            .await?;
    } else {
        ctx.say(format!("注文 `{}` が見つかりません。", id)).await?;
    }
    Ok(())
}

/// 注文をキャンセルします
#[poise::command(slash_command)]
async fn cancel_order(
    ctx: PoiseContext<'_>,
    #[description = "注文ID"] id: u32,
) -> Result<(), anyhow::Error> {
    if let Some(order) = ctx.data().cancel_order(id).await {
        ctx.say(format!("注文 `{}` をキャンセルしました。", order.id))
            .await?;
    } else {
        ctx.say(format!("注文 `{}` が見つかりません。", id)).await?;
    }
    Ok(())
}

/// 注文の優先度を更新します
#[poise::command(slash_command)]
async fn update_order_priority(
    ctx: PoiseContext<'_>,
    #[description = "注文ID"] id: u32,
    #[description = "優先注文にするか"] is_priority: bool,
) -> Result<(), anyhow::Error> {
    if let Some(order) = ctx.data().update_order_priority(id, is_priority).await {
        ctx.say(format!(
            "注文 `{}` の優先度を `{}` に更新しました。",
            order.id, is_priority
        ))
        .await?;
    } else {
        ctx.say(format!("注文 `{}` が見つかりません。", id)).await?;
    }
    Ok(())
}

/// フレーバーの設定を取得します
#[poise::command(slash_command)]
async fn get_flavor_configs(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    let data = ctx.data().data().await;
    let mut response = String::new();
    response.push_str("## フレーバー設定一覧\n");
    for (flavor, config) in data.flavor_configs.iter() {
        response.push_str(&format!(
            "- **{}**: 調理時間: {}分, バッチ生産数: {}\n",
            flavor, config.cooking_time_minutes, config.quantity_per_batch
        ));
    }
    ctx.say(response).await?;
    Ok(())
}

/// フレーバーの設定を更新します
#[poise::command(slash_command)]
async fn set_flavor_config(
    ctx: PoiseContext<'_>,
    #[description = "フレーバー"] flavor: FlavorChoice,
    #[description = "調理時間(分)"] cooking_time_minutes: u32,
    #[description = "バッチあたりの生産数"] quantity_per_batch: u32,
) -> Result<(), anyhow::Error> {
    let flavor: Flavor = flavor.into();
    let config = FlavorConfig {
        cooking_time_minutes,
        quantity_per_batch,
    };
    ctx.data().set_flavor_config(flavor, config).await;
    ctx.say(format!("`{}` の設定を更新しました。", flavor))
        .await?;
    Ok(())
}
