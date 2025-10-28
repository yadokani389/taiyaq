use axum::http::StatusCode;
use bot_sdk_line::webhook_line::models::{CallbackRequest, Event, MessageContent, Source};

use crate::app::AppRegistry;
use crate::line::commands;

pub async fn line_handler(registry: &AppRegistry, req: CallbackRequest) -> Result<(), StatusCode> {
    for e in req.events {
        match e {
            // テキストメッセージの処理
            Event::MessageEvent(message_event) => {
                if let MessageContent::TextMessageContent(text_message) = *message_event.message {
                    let reply_token = message_event.reply_token.ok_or(StatusCode::BAD_REQUEST)?;
                    let user_id = message_event.source.and_then(|source| match *source {
                        Source::UserSource(user_source) => user_source.user_id,
                        Source::GroupSource(group_source) => Some(group_source.group_id),
                        Source::RoomSource(room_source) => Some(room_source.room_id),
                    });
                    let user_message = text_message.text.trim();

                    // コマンド判定（先頭が「!」かどうか）
                    if user_message.starts_with("!") {
                        commands::handle_command(registry, reply_token, user_message, user_id)
                            .await;
                    } else {
                        let reply_text = "あつあつのうちに取りに来てね！🔥".to_string();
                        commands::send_text_reply(registry, reply_token, reply_text).await;
                    }
                }
            }
            // Postbackイベントの処理
            Event::PostbackEvent(postback_event) => {
                let reply_token = postback_event.reply_token.ok_or(StatusCode::BAD_REQUEST)?;
                let user_id = postback_event.source.and_then(|source| match *source {
                    Source::UserSource(user_source) => user_source.user_id,
                    Source::GroupSource(group_source) => Some(group_source.group_id),
                    Source::RoomSource(room_source) => Some(room_source.room_id),
                });
                let postback_data = postback_event.postback.data.as_str();

                commands::handle_postback(registry, reply_token, postback_data, user_id).await;
            }
            _ => {
                println!("Unhandled event type");
            }
        }
    }

    Ok(())
}
