use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use bot_sdk_line::{
    messaging_api_line::{
        apis::MessagingApiApi,
        models::{Message, ReplyMessageRequest, TextMessageV2},
    },
    webhook_line::models::{CallbackRequest, Event, MessageContent},
};

use crate::{
    api::{
        handler::{display::get_order_details, staff::add_notification},
        model::AddNotificationRequest,
    },
    data::{AppRegistry, NotifyChannel},
};

/// POST /line_callback
pub async fn line_callback(
    State(registry): State<AppRegistry>,
    Json(req): Json<CallbackRequest>,
) -> StatusCode {
    println!("req: {req:#?}");

    for e in req.events {
        if let Event::MessageEvent(message_event) = e
            && let MessageContent::TextMessageContent(text_message) = *message_event.message
        {
            let reply_token = message_event.reply_token.unwrap();
            let user_message = text_message.text.trim();
            let reply_text;

            // Example: "notify 123 user@example.com" or "notify 123 line_user_id"
            if user_message.starts_with("notify ") {
                let parts: Vec<&str> = user_message.split_whitespace().collect();
                if parts.len() == 3 {
                    if let Ok(order_id) = parts[1].parse::<u32>() {
                        let target = parts[2].to_string();
                        let channel = if target.contains('@') {
                            NotifyChannel::Email
                        } else {
                            NotifyChannel::Line
                        };

                        let add_notification_payload = AddNotificationRequest {
                            channel,
                            target: target.clone(),
                        };

                        // Call the internal add_notification handler logic
                        let result = add_notification(
                            State(registry.clone()),
                            Path(order_id),
                            Json(add_notification_payload),
                        )
                        .await;

                        match result {
                            Ok(_) => {
                                reply_text = format!(
                                    "Notification set for order {} to {}.",
                                    order_id, target
                                )
                            }
                            Err(_) => {
                                reply_text =
                                    format!("Failed to set notification for order {}.", order_id)
                            }
                        }
                    } else {
                        reply_text = "Invalid order ID for notification.".to_string();
                    }
                } else {
                    reply_text = "Usage: notify <order_id> <email_or_line_id>".to_string();
                }
            }
            // Example: "status 123"
            else if user_message.starts_with("status ") {
                let parts: Vec<&str> = user_message.split_whitespace().collect();
                if parts.len() == 2 {
                    if let Ok(order_id) = parts[1].parse::<u32>() {
                        // Call the internal get_order_details handler logic
                        let result =
                            get_order_details(State(registry.clone()), Path(order_id)).await;

                        match result {
                            Ok(Json(details)) => {
                                reply_text = format!(
                                    "Order {}: Status is {:?}. Estimated wait: {}.",
                                    details.id,
                                    details.status,
                                    details
                                        .estimated_wait_minutes
                                        .map_or("N/A".to_string(), |m| format!("{} minutes", m))
                                );
                            }
                            Err(StatusCode::NOT_FOUND) => {
                                reply_text = format!("Order {} not found.", order_id)
                            }
                            Err(_) => {
                                reply_text = format!("Failed to get status for order {}.", order_id)
                            }
                        }
                    } else {
                        reply_text = "Invalid order ID for status inquiry.".to_string();
                    }
                } else {
                    reply_text = "Usage: status <order_id>".to_string();
                }
            } else {
                reply_text = format!(
                    "You said: {}. Try 'notify <order_id> <target>' or 'status <order_id>'.",
                    user_message
                );
            }

            let reply_message_request = ReplyMessageRequest {
                reply_token,
                messages: vec![Message::TextMessageV2(TextMessageV2 {
                    text: reply_text,
                    ..Default::default()
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
            println!("{:#?}", result);
        };
    }
    StatusCode::OK
}
