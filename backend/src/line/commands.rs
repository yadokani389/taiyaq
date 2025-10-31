use bot_sdk_line::messaging_api_line::{
    apis::MessagingApiApi,
    models::{
        Action, ButtonsTemplate, ConfirmTemplate, ImageMessage, Message, PostbackAction,
        ReplyMessageRequest, TemplateMessage, TextMessageV2, template::Template,
    },
};

use crate::{
    api::model::AddNotificationRequest,
    app::AppRegistry,
    data::{NotifyChannel, OrderStatus},
};

// ========== 公開API: イベントハンドラー ==========

/// コマンドを処理
pub async fn handle_command(
    registry: &AppRegistry,
    reply_token: String,
    command: &str,
    user_id: Option<String>,
) {
    if let Some(order_id_str) = command.strip_prefix("!adding_notification:") {
        handle_adding_notification(registry, reply_token, order_id_str.trim(), user_id).await;
    } else {
        send_reply(
            registry,
            reply_token,
            vec![create_text_message(
                "不明なコマンドです。\nリッチメニューから操作してください。".to_string(),
            )],
        )
        .await;
    }
}

/// 通常のテキストメッセージを処理
pub async fn handle_text_message(registry: &AppRegistry, reply_token: String, _text: &str) {
    send_reply(
        registry,
        reply_token,
        vec![create_text_message(
            "たいやきくんはダンスが上手！".to_string(),
        )],
    )
    .await;
}

/// Postbackイベントを処理
pub async fn handle_postback(
    registry: &AppRegistry,
    reply_token: String,
    postback_data: &str,
    user_id: Option<String>,
) {
    // 注文状況確認
    if let Some(order_id_str) = postback_data.strip_prefix("check_order_")
        && let Ok(order_id) = order_id_str.parse::<u32>()
    {
        handle_check_order_status(registry, reply_token, order_id).await;
        return;
    }

    // 通知登録確認
    if let Some(order_id_str) = postback_data.strip_prefix("notify_confirm_")
        && let Ok(order_id) = order_id_str.parse::<u32>()
    {
        handle_notification_confirm(registry, reply_token, order_id, user_id).await;
        return;
    }

    // 通知登録キャンセル
    if postback_data.starts_with("notify_cancel_") {
        send_reply(
            registry,
            reply_token,
            vec![create_text_message(
                "通知の登録をキャンセルしました。".to_string(),
            )],
        )
        .await;
        return;
    }

    // アクセス画像
    if postback_data == "action=show_access" {
        send_access_image(registry, reply_token).await;
        return;
    }

    // 待ち時間表示
    if postback_data == "action=show_waittime" {
        handle_show_waittime(registry, reply_token).await;
        return;
    }

    // その他の定型アクション
    let reply_text = get_static_reply_text(postback_data);
    send_reply(registry, reply_token, vec![create_text_message(reply_text)]).await;
}

// ========== プライベート: ビジネスロジック ==========

/// 注文状況を確認
async fn handle_check_order_status(registry: &AppRegistry, reply_token: String, order_id: u32) {
    if let Some(details) = registry.get_order_details(order_id).await {
        let reply_text = format_order_details(&details);
        send_reply(registry, reply_token, vec![create_text_message(reply_text)]).await;
    } else {
        send_error_message(registry, reply_token, order_id, "が見つかりません").await;
    }
}

/// 通知登録確認を処理
async fn handle_notification_confirm(
    registry: &AppRegistry,
    reply_token: String,
    order_id: u32,
    user_id: Option<String>,
) {
    let Some(user_id) = user_id else {
        send_reply(
            registry,
            reply_token,
            vec![create_text_message(
                "❌ ユーザー情報の取得に失敗しました。".to_string(),
            )],
        )
        .await;
        return;
    };

    let payload = AddNotificationRequest {
        channel: NotifyChannel::Line,
        target: user_id,
    };

    if registry.add_notification(order_id, payload).await.is_some() {
        let buttons_template = create_notification_success_template(order_id);
        send_reply(
            registry,
            reply_token,
            vec![create_template_message(
                Template::ButtonsTemplate(buttons_template),
                "通知登録完了",
            )],
        )
        .await;
    } else {
        send_reply(
            registry,
            reply_token,
            vec![create_text_message(
                "❌ エラー：通知の登録に失敗しました。".to_string(),
            )],
        )
        .await;
    }
}

/// 通知追加コマンドを処理
async fn handle_adding_notification(
    registry: &AppRegistry,
    reply_token: String,
    order_id_str: &str,
    user_id: Option<String>,
) {
    let Ok(order_id) = order_id_str.parse::<u32>() else {
        send_reply(
            registry,
            reply_token,
            vec![create_text_message(
                "❌ 不正な注文番号です。\n半角数字で入力してください。\n例: !adding_notification: 123"
                    .to_string(),
            )],
        )
        .await;
        return;
    };

    if user_id.is_none() {
        send_reply(
            registry,
            reply_token,
            vec![create_text_message(
                "❌ ユーザー情報の取得に失敗しました。".to_string(),
            )],
        )
        .await;
        return;
    }

    match registry.get_order_details(order_id).await {
        Some(details)
            if !matches!(
                details.status,
                OrderStatus::Completed | OrderStatus::Cancelled
            ) =>
        {
            let confirm_template = create_notification_confirm_template(&details);
            send_reply(
                registry,
                reply_token,
                vec![create_template_message(
                    Template::ConfirmTemplate(confirm_template),
                    "通知登録の確認",
                )],
            )
            .await;
        }
        Some(_) => {
            send_reply(
                registry,
                reply_token,
                vec![create_text_message(format!(
                    "❌ 注文 {} はすでに完了/キャンセルされています。",
                    order_id
                ))],
            )
            .await;
        }
        None => {
            send_error_message(registry, reply_token, order_id, "が見つかりません").await;
        }
    }
}

/// 待ち時間を表示
async fn handle_show_waittime(registry: &AppRegistry, reply_token: String) {
    let wait_times = registry.get_current_wait_times().await;
    let reply_text = format_wait_times(&wait_times);
    send_reply(registry, reply_token, vec![create_text_message(reply_text)]).await;
}

// ========== ヘルパー関数：メッセージフォーマット ==========

/// OrderDetailsResponse をユーザー向けにフォーマット
fn format_order_details(details: &crate::api::model::OrderDetailsResponse) -> String {
    let status_text = match details.status {
        OrderStatus::Waiting => "⏳ 待機中",
        OrderStatus::Cooking => "🍳 調理中",
        OrderStatus::Ready => "✅ 受け取り準備完了",
        OrderStatus::Completed => "🎉 完了",
        OrderStatus::Cancelled => "❌ キャンセル",
    };

    let items_str = details
        .items
        .iter()
        .map(|item| format!("  ・{} x{}", item.flavor, item.quantity))
        .collect::<Vec<_>>()
        .join("\n");

    let ordered_at_str = details.ordered_at.format("%Y年%m月%d日 %H:%M").to_string();

    let wait_time_str = details
        .estimated_wait_minutes
        .map_or("N/A".to_string(), |m| format!("{} 分", m));

    format!(
        "📦 注文 #{}\n\n【現在の状態】\n{}\n\n【予想待ち時間】\n{}\n\n【商品】\n{}\n\n【注文時刻】\n{}",
        details.id, status_text, wait_time_str, items_str, ordered_at_str
    )
}

/// 待ち時間をフォーマット
fn format_wait_times(wait_times: &crate::api::model::WaitTimeResponse) -> String {
    let mut lines = vec!["⏱️ 現在の待ち時間".to_string(), "".to_string()];

    for (flavor, time) in &wait_times.wait_times {
        let time_str = time.map_or("提供なし".to_string(), |t| {
            if t == 0 {
                "すぐに提供できます".to_string()
            } else {
                format!("約{}分", t)
            }
        });
        lines.push(format!("【{}】\n{}", flavor, time_str));
    }

    lines.join("\n")
}

/// 静的な返信テキストを取得
fn get_static_reply_text(postback_data: &str) -> String {
    match postback_data {
        "action=register_notification" => {
            "注文番号を半角数字で続いて入力↓\n例:'!adding_notification: 123'".into()
        }
        "action=show_menu" => {
            "🐟メニュー☆彡\n- つぶあん (200円)\n- カスタード (200円)\n- 栗きんとん (200円)".into()
        }
        _ => format!("不明な操作です: {}", postback_data),
    }
}

// ========== ヘルパー関数：メッセージ作成 ==========

/// テキストメッセージを作成
fn create_text_message(text: String) -> Message {
    Message::TextMessageV2(TextMessageV2 {
        r#type: None,
        quick_reply: None,
        sender: None,
        text,
        substitution: None,
        quote_token: None,
    })
}

/// テンプレートメッセージを作成
fn create_template_message(template: Template, alt_text: &str) -> Message {
    Message::TemplateMessage(TemplateMessage {
        r#type: None,
        alt_text: alt_text.to_string(),
        quick_reply: None,
        sender: None,
        template: Box::new(template),
    })
}

/// 通知登録成功時のボタンテンプレートを作成
fn create_notification_success_template(order_id: u32) -> ButtonsTemplate {
    ButtonsTemplate {
        r#type: None,
        thumbnail_image_url: None,
        image_aspect_ratio: None,
        image_size: None,
        image_background_color: None,
        title: Some("通知登録完了".to_string()),
        text: format!(
            "✅ 注文 #{} の通知を登録しました！\n準備ができたらメッセージをお送りします。",
            order_id
        ),
        default_action: None,
        actions: vec![Action::PostbackAction(PostbackAction {
            r#type: None,
            label: Some("📦 注文状況を確認".to_string()),
            data: Some(format!("check_order_{}", order_id)),
            display_text: Some("注文状況を確認".to_string()),
            text: None,
            input_option: None,
            fill_in_text: None,
        })],
    }
}

/// 通知登録確認のテンプレートを作成（OrderDetailsResponse 版）
fn create_notification_confirm_template(
    details: &crate::api::model::OrderDetailsResponse,
) -> ConfirmTemplate {
    let items_str = details
        .items
        .iter()
        .map(|item| format!("・{} x{}", item.flavor, item.quantity))
        .collect::<Vec<_>>()
        .join("\n");

    let ordered_at_str = details
        .ordered_at
        .format("%Y年%m月%d日 %H:%M:%S")
        .to_string();

    ConfirmTemplate {
        r#type: None,
        text: format!(
            "📝 注文 #{} の通知設定\n\n以下の注文で通知を登録しますか？\n\n【商品】\n{}\n\n【注文時刻】\n{}",
            details.id, items_str, ordered_at_str
        ),
        actions: vec![
            Action::PostbackAction(PostbackAction {
                r#type: None,
                label: Some("はい".to_string()),
                data: Some(format!("notify_confirm_{}", details.id)),
                display_text: Some("通知を登録しました".to_string()),
                text: None,
                input_option: None,
                fill_in_text: None,
            }),
            Action::PostbackAction(PostbackAction {
                r#type: None,
                label: Some("いいえ".to_string()),
                data: Some(format!("notify_cancel_{}", details.id)),
                display_text: Some("キャンセルしました".to_string()),
                text: None,
                input_option: None,
                fill_in_text: None,
            }),
        ],
    }
}

// ========== 汎用メッセージ送信関数 ==========

/// メッセージを返信
async fn send_reply(registry: &AppRegistry, reply_token: String, messages: Vec<Message>) {
    let reply_message_request = ReplyMessageRequest {
        reply_token,
        messages,
        notification_disabled: Some(false),
    };

    if let Err(e) = registry
        .line
        .lock()
        .await
        .messaging_api_client
        .reply_message(reply_message_request)
        .await
    {
        eprintln!("Failed to send reply: {:?}", e);
    }
}

/// エラーメッセージを送信
async fn send_error_message(
    registry: &AppRegistry,
    reply_token: String,
    order_id: u32,
    reason: &str,
) {
    send_reply(
        registry,
        reply_token,
        vec![create_text_message(format!(
            "❌ 注文 {} {}。",
            order_id, reason
        ))],
    )
    .await;
}

/// アクセス画像を返信
async fn send_access_image(registry: &AppRegistry, reply_token: String) {
    let image_url = std::env::var("ACCESS_IMAGE_URL").unwrap_or(
        "https://raw.githubusercontent.com/yadokani389/taiyaq/main/backend/assets/access.png"
            .into(),
    );

    send_reply(
        registry,
        reply_token,
        vec![Message::ImageMessage(ImageMessage {
            r#type: None,
            quick_reply: None,
            sender: None,
            original_content_url: image_url.clone(),
            preview_image_url: image_url,
        })],
    )
    .await;
}
