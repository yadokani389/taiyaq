use bot_sdk_line::{
    messaging_api_line::{
        apis::MessagingApiApi,
        models::{
            postback_action::InputOption, template::Template, Action, ConfirmTemplate, Message,
            PostbackAction, ReplyMessageRequest, TemplateMessage, TextMessageV2,
        },
    },
};

use crate::app::AppRegistry;

/// コマンドを処理
pub async fn handle_command(registry: &AppRegistry, reply_token: String, command: &str) {
    match command {
        "!notification" => {
            show_notification_button(registry, reply_token).await;
        }
        "!access" => {
            let reply_text = "📍アクセス\n校内マップ: https://example.com/map\n※実際のURLに置き換えてください".to_string();
            send_text_reply(registry, reply_token, reply_text).await;
        }
        "!menu" => {
            let reply_text = "🐟メニュー☆彡\n- つぶあん (200円)\n- カスタード (200円)\n- いも（あんこ） (200円)".to_string();
            send_text_reply(registry, reply_token, reply_text).await;
        }
        "!help" => {
            let reply_text = "📖 HELP\n\n【よくある質問】\n\nQ. グループへの参加ができない\nA. グループへの参加は許可されていません。\n\nQ. 操作方法がわからない\nA. 注文受付のスタッフにお声がけください。\n\n【コマンド一覧】\n!menu - メニューを表示\n!access - アクセス情報を表示\n!notification - 通知登録\n!help - このヘルプを表示".to_string();
            send_text_reply(registry, reply_token, reply_text).await;
        }
        cmd if cmd.starts_with("!adding_notification:") => {
            handle_adding_notification(registry, reply_token, cmd).await;
        }
        _ => {
            let reply_text = "不明なコマンドです。\n「!help」でコマンド一覧を確認できます。".to_string();
            send_text_reply(registry, reply_token, reply_text).await;
        }
    }
}

/// Postbackイベントを処理
pub async fn handle_postback(registry: &AppRegistry, reply_token: String, postback_data: &str) {
    let reply_text = match postback_data {
        "notification_register" => {
            "注文番号を半角数字で続いて入力↓\n例:\"!adding_notification: 123\"".to_string()
        }
        "notification_cancel" => "キャンセルされました".to_string(),
        _ => "不明な操作です".to_string(),
    };

    send_text_reply(registry, reply_token, reply_text).await;
}

/// 通知追加コマンドを処理
async fn handle_adding_notification(registry: &AppRegistry, reply_token: String, command: &str) {
    let order_id_str = command
        .strip_prefix("!adding_notification:")
        .unwrap_or("")
        .trim();

    match order_id_str.parse::<u32>() {
        Ok(order_id) => {
            // TODO: 実際の通知登録処理を実装
            // registry.add_notification(order_id, ...).await;
            let reply_text = format!("✅ 注文ID {} の通知を登録しました！\n完了時にメッセージをお送りします。", order_id);
            send_text_reply(registry, reply_token, reply_text).await;
        }
        Err(_) => {
            let reply_text =
                "❌ 不正な注文番号です。\n半角数字で入力してください。\n例: !adding_notification: 123"
                    .to_string();
            send_text_reply(registry, reply_token, reply_text).await;
        }
    }
}

/// 通知登録用のボタンテンプレートを表示
async fn show_notification_button(registry: &AppRegistry, reply_token: String) {
    let confirms = ConfirmTemplate {
        r#type: None,
        text: "通知登録を行いますか？\n注文番号を続けて入力してください。".to_string(),
        actions: vec![
            Action::PostbackAction(PostbackAction {
                r#type: None,
                label: Some("通知登録".to_string()),
                data: Some("notification_register".to_string()),
                display_text: None,
                text: None,
                input_option: Some(InputOption::OpenKeyboard),
                fill_in_text: Some("!adding_notification: ".to_string()),
            }),
            Action::PostbackAction(PostbackAction {
                r#type: None,
                label: Some("キャンセル".to_string()),
                data: Some("notification_cancel".to_string()),
                display_text: Some("キャンセルしました".to_string()),
                text: None,
                input_option: None,
                fill_in_text: None,
            }),
        ],
    };

    send_template_reply(registry, reply_token, confirms, "通知登録").await;
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