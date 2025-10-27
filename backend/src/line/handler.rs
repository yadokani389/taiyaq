use axum::http::StatusCode;
use bot_sdk_line::{webhook_line::models::{CallbackRequest, Event, MessageContent}};

use crate::app::AppRegistry;
use crate::line::commands;

pub async fn line_handler(
    registry: &AppRegistry,
    req: CallbackRequest
) -> Result<(), StatusCode> {
    for e in req.events {
        match e {
            // テキストメッセージの処理
            Event::MessageEvent(message_event) => {
                if let MessageContent::TextMessageContent(text_message) = *message_event.message {
                    let reply_token = message_event.reply_token.unwrap();
                    let user_message = text_message.text.trim();

                    // コマンド判定（先頭が「!」かどうか）
                    if user_message.starts_with("!") {
                        commands::handle_command(registry, reply_token, user_message).await;
                    } else {
                        let reply_text = "あつあつのうちに取りに来てね！🔥".to_string();
                        commands::send_text_reply(registry, reply_token, reply_text).await;
                    }
                }
            }
            // Postbackイベントの処理
            Event::PostbackEvent(postback_event) => {
                let reply_token = postback_event.reply_token.unwrap();
                let postback_data = postback_event.postback.data.as_str();
                
                commands::handle_postback(registry, reply_token, postback_data).await;
            }
            _ => {
                println!("Unhandled event type");
            }
        }
    }

    Ok(())
}