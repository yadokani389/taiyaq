use bot_sdk_line::messaging_api_line::{
    apis::MessagingApiApi,
    models::{
        Action, ConfirmTemplate, ImageMessage, Message, PostbackAction, ReplyMessageRequest,
        TemplateMessage, TextMessageV2, template::Template,
    },
};

use crate::{
    api::model::AddNotificationRequest,
    app::AppRegistry,
    data::{NotifyChannel, OrderStatus},
};

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
        let reply_text = "不明なコマンドです。\nリッチメニューから操作してください。".to_string();
        send_text_reply(registry, reply_token, reply_text).await;
    }
}

/// Postbackイベントを処理
pub async fn handle_postback(
    registry: &AppRegistry,
    reply_token: String,
    postback_data: &str,
    user_id: Option<String>,
) {
    // 通知登録の確認ボタンからのPostback
    if let Some(order_id_str) = postback_data.strip_prefix("notify_confirm_")
        && let Ok(order_id) = order_id_str.parse::<u32>()
    {
        // user_id が取得できない場合はエラー
        let Some(user_id) = user_id else {
            let reply_text = "❌ ユーザー情報の取得に失敗しました。".to_string();
            send_text_reply(registry, reply_token, reply_text).await;
            return;
        };

        // 通知登録処理
        let payload = AddNotificationRequest {
            channel: NotifyChannel::Line,
            target: user_id,
        };

        if registry.add_notification(order_id, payload).await.is_some() {
            let reply_text = format!(
                "✅ 注文 #{} の通知を登録しました！\n準備ができたらメッセージをお送りします。",
                order_id
            );
            send_text_reply(registry, reply_token, reply_text).await;
        } else {
            let reply_text = "❌ エラー：通知の登録に失敗しました。".to_string();
            send_text_reply(registry, reply_token, reply_text).await;
        }
        return;
    }

    // 通知登録のキャンセルボタンからのPostback
    if postback_data.starts_with("notify_cancel_") {
        let reply_text = "通知の登録をキャンセルしました。".to_string();
        send_text_reply(registry, reply_token, reply_text).await;
        return;
    }

    // アクセス画像は特別処理
    if postback_data == "action=show_access" {
        send_access_image_reply(registry, reply_token).await;
        return;
    }

    let reply_text = match postback_data {
        "action=register_notification" => "注文番号を半角数字で続いて入力↓\n例:'!adding_notification: 123'".into(),
        "action=show_menu" => "🐟メニュー☆彡\n- つぶあん (200円)\n- カスタード (200円)\n- 栗きんとん (200円)".into(),
        "action=show_help" => "📖 HELP\n\n【よくある質問】\n\nQ. 操作方法がわからない\nA. 注文受付のスタッフにお声がけください。\n\n【使い方】\nリッチメニューから各機能を選択してください。".into(),
        "notification_cancel" => "キャンセルされました".into(),
        _ => format!("不明な操作です: {}", postback_data),
    };

    send_text_reply(registry, reply_token, reply_text).await;
}

/// 通知追加コマンドを処理
async fn handle_adding_notification(
    registry: &AppRegistry,
    reply_token: String,
    order_id_str: &str,
    user_id: Option<String>,
) {
    let Ok(order_id) = order_id_str.parse::<u32>() else {
        let reply_text =
            "❌ 不正な注文番号です。\n半角数字で入力してください。\n例: !adding_notification: 123"
                .to_string();
        send_text_reply(registry, reply_token, reply_text).await;
        return;
    };
    // user_id が取得できない場合はエラー
    if user_id.is_none() {
        let reply_text = "❌ ユーザー情報の取得に失敗しました。".to_string();
        send_text_reply(registry, reply_token, reply_text).await;
        return;
    };

    // 注文情報を取得
    let data = registry.data().await;
    let Some(order) = data.orders.iter().find(|o| o.id == order_id).cloned() else {
        let reply_text = format!("❌ 注文 {} が見つかりません。", order_id);
        send_text_reply(registry, reply_token, reply_text).await;
        return;
    };
    drop(data);

    // 注文がすでに完了/キャンセルされている場合
    if order.status == OrderStatus::Completed || order.status == OrderStatus::Cancelled {
        let reply_text = format!("❌ 注文 {} はすでに完了/キャンセルされています。", order_id);
        send_text_reply(registry, reply_token, reply_text).await;
        return;
    }

    // 注文内容を整形
    let items_str = order
        .items
        .iter()
        .map(|item| format!("・{} x{}", item.flavor, item.quantity))
        .collect::<Vec<_>>()
        .join("\n");

    let ordered_at_str = order.ordered_at.format("%Y年%m月%d日 %H:%M:%S").to_string();

    // 確認メッセージを作成
    let confirm_text = format!(
        "📝 注文 #{} の通知設定\n\n以下の注文で通知を登録しますか？\n\n【商品】\n{}\n\n【注文時刻】\n{}",
        order.id, items_str, ordered_at_str
    );

    let confirms = ConfirmTemplate {
        r#type: None,
        text: confirm_text,
        actions: vec![
            Action::PostbackAction(PostbackAction {
                r#type: None,
                label: Some("はい".to_string()),
                data: Some(format!("notify_confirm_{}", order_id)),
                display_text: Some("通知を登録しました".to_string()),
                text: None,
                input_option: None,
                fill_in_text: None,
            }),
            Action::PostbackAction(PostbackAction {
                r#type: None,
                label: Some("いいえ".to_string()),
                data: Some(format!("notify_cancel_{}", order_id)),
                display_text: Some("キャンセルしました".to_string()),
                text: None,
                input_option: None,
                fill_in_text: None,
            }),
        ],
    };

    send_template_reply(registry, reply_token, confirms, "通知登録の確認").await;
}

/// テンプレートメッセージを返信（汎用）
async fn send_template_reply(
    registry: &AppRegistry,
    reply_token: String,
    template: ConfirmTemplate,
    alt_text: &str,
) {
    let template_message = TemplateMessage {
        r#type: None,
        alt_text: alt_text.to_string(),
        quick_reply: None,
        sender: None,
        template: Box::new(Template::ConfirmTemplate(template)),
    };

    let reply_message_request = ReplyMessageRequest {
        reply_token,
        messages: vec![Message::TemplateMessage(template_message)],
        notification_disabled: Some(false),
    };

    let result = registry
        .line
        .lock()
        .await
        .messaging_api_client
        .reply_message(reply_message_request)
        .await;

    if let Err(e) = result {
        eprintln!("Failed to send template reply: {:?}", e);
    }
}

/// テキストメッセージを返信
pub async fn send_text_reply(registry: &AppRegistry, reply_token: String, text: String) {
    let reply_message_request = ReplyMessageRequest {
        reply_token,
        messages: vec![Message::TextMessageV2(TextMessageV2 {
            r#type: None,
            quick_reply: None,
            sender: None,
            text,
            substitution: None,
            quote_token: None,
        })],
        notification_disabled: Some(false),
    };

    let result = registry
        .line
        .lock()
        .await
        .messaging_api_client
        .reply_message(reply_message_request)
        .await;

    if let Err(e) = result {
        eprintln!("Failed to send text reply: {:?}", e);
    }
}

async fn send_access_image_reply(registry: &AppRegistry, reply_token: String) {
    let image_url = std::env::var("ACCESS_IMAGE_URL").unwrap_or(
        "https://raw.githubusercontent.com/yadokani389/taiyaq/main/backend/assets/access.png"
            .into(),
    );

    let image_message = ImageMessage {
        r#type: None,
        quick_reply: None,
        sender: None,
        original_content_url: image_url.clone(),
        preview_image_url: image_url,
    };

    let reply_message_request = ReplyMessageRequest {
        reply_token,
        messages: vec![Message::ImageMessage(image_message)],
        notification_disabled: Some(false),
    };

    let result = registry
        .line
        .lock()
        .await
        .messaging_api_client
        .reply_message(reply_message_request)
        .await;

    if let Err(e) = result {
        eprintln!("Failed to send access image reply: {:?}", e);
    }
}
