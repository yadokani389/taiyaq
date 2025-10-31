use axum::http::StatusCode;
use bot_sdk_line::webhook_line::models::{CallbackRequest, Event, MessageContent, Source};

use crate::app::AppRegistry;
use crate::line::commands;

pub async fn line_handler(registry: &AppRegistry, req: CallbackRequest) -> Result<(), StatusCode> {
    for e in req.events {
        match e {
            // テキストメッセージの処理
            Event::MessageEvent(message_event) => {
                let reply_token = message_event.reply_token.ok_or(StatusCode::BAD_REQUEST)?;
                let user_id = extract_user_id(message_event.source);

                if let MessageContent::TextMessageContent(text_message) = *message_event.message {
                    let user_message = text_message.text.trim();

                    // コマンド判定（先頭が「!」かどうか）
                    if user_message.starts_with("!") {
                        commands::handle_command(registry, reply_token, user_message, user_id)
                            .await;
                    } else {
                        commands::handle_text_message(registry, reply_token, user_message).await;
                    }
                }
            }
            // Postbackイベントの処理
            Event::PostbackEvent(postback_event) => {
                let reply_token = postback_event.reply_token.ok_or(StatusCode::BAD_REQUEST)?;
                let user_id = extract_user_id(postback_event.source);
                let postback_data = postback_event.postback.data.as_str();

                commands::handle_postback(registry, reply_token, postback_data, user_id).await;
            }
            _ => {
                println!("Unhandled event type: {:?}", e);
            }
        }
    }

    Ok(())
}

/// ソースからユーザーIDを抽出
fn extract_user_id(source: Option<Box<Source>>) -> Option<String> {
    source.and_then(|s| match *s {
        Source::UserSource(user) => user.user_id,
        Source::GroupSource(group) => Some(group.group_id),
        Source::RoomSource(room) => Some(room.room_id),
    })
}
