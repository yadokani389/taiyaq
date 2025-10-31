use bot_sdk_line::client::LINE;
use bot_sdk_line::messaging_api_line::apis::MessagingApiApi;
use bot_sdk_line::messaging_api_line::models::{Message, PushMessageRequest, TextMessageV2};
use tokio::sync::MutexGuard;

pub mod commands;
pub mod handler;

pub async fn send_notification(line: MutexGuard<'_, LINE>, user_id: String, message: String) {
    let push_request = PushMessageRequest {
        to: user_id.clone(), // LINE user_id
        messages: vec![Message::TextMessageV2(TextMessageV2 {
            r#type: None,
            quick_reply: None,
            sender: None,
            text: message.clone(),
            substitution: None,
            quote_token: None,
        })],
        notification_disabled: Some(false),
        custom_aggregation_units: None,
    };

    match line
        .messaging_api_client
        .push_message(push_request, None)
        .await
    {
        Ok(_) => {
            println!("✅ LINE notification sent to user {}", user_id);
        }
        Err(e) => {
            eprintln!("Failed to send LINE notification to {}: {:?}", user_id, e);
        }
    }
}
