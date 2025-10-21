use crate::{
    api::model::{
        AddNotificationRequest, CreateOrderRequest, DisplayOrder, DisplayOrdersResponse,
        OrderDetailsResponse, UpdateProductionRequest, UpdateProductionResponse,
    },
    data::{AppRegistry, Notify, NotifyChannel, Order, OrderStatus},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::Deserialize;

use bot_sdk_line::{
    // Added for LINE callback
    messaging_api_line::{
        apis::MessagingApiApi,
        models::{Message, ReplyMessageRequest, TextMessageV2},
    },
    webhook_line::models::{CallbackRequest, Event, MessageContent},
};

//==// User & Display API Handlers //==//

/// GET /api/orders/display
pub async fn get_display_orders(
    State(registry): State<AppRegistry>,
) -> Json<DisplayOrdersResponse> {
    let orders = registry.orders.lock().await;
    let ready = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Ready)
        .map(|o| DisplayOrder { id: o.id })
        .collect();
    let cooking = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Cooking)
        .map(|o| DisplayOrder { id: o.id })
        .collect();
    Json(DisplayOrdersResponse { ready, cooking })
}

/// GET /api/orders/{id}
pub async fn get_order_details(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<OrderDetailsResponse>, StatusCode> {
    let orders = registry.orders.lock().await;
    if let Some(order) = orders.iter().find(|o| o.id == id) {
        let estimated_wait_minutes = if order.status == OrderStatus::Waiting {
            // Simplified estimation logic: 5 minutes per waiting order ahead of this one.
            let position = orders
                .iter()
                .filter(|o| o.status == OrderStatus::Waiting && o.ordered_at < order.ordered_at)
                .count();
            Some((position as i64 + 1) * 5)
        } else {
            None
        };
        Ok(Json(OrderDetailsResponse {
            id: order.id,
            status: order.status,
            estimated_wait_minutes,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

//==// Staff API Handlers //==//

#[derive(Deserialize, Debug)]
pub struct StaffOrdersQuery {
    #[serde(default, deserialize_with = "csv_to_order_status")]
    status: Vec<OrderStatus>,
}

// Custom deserializer for comma-separated order status strings
fn csv_to_order_status<'de, D>(deserializer: D) -> Result<Vec<OrderStatus>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        return Ok(Vec::new());
    }
    s.split(',')
        .filter(|s| !s.trim().is_empty()) // Filter out empty strings resulting from trailing commas etc.
        .map(|s_trim| {
            let s = s_trim.trim();
            match s {
                "waiting" => Ok(OrderStatus::Waiting),
                "cooking" => Ok(OrderStatus::Cooking),
                "ready" => Ok(OrderStatus::Ready),
                "completed" => Ok(OrderStatus::Completed),
                "cancelled" => Ok(OrderStatus::Cancelled),
                _ => Err(serde::de::Error::custom(format!(
                    "invalid order status: '{}'",
                    s
                ))),
            }
        })
        .collect()
}

/// GET /api/staff/orders
pub async fn get_staff_orders(
    State(registry): State<AppRegistry>,
    Query(query): Query<StaffOrdersQuery>,
) -> Json<Vec<Order>> {
    let orders = registry.orders.lock().await;
    let filtered_orders = if query.status.is_empty() {
        orders.clone()
    } else {
        orders
            .iter()
            .filter(|o| query.status.contains(&o.status))
            .cloned()
            .collect()
    };
    Json(filtered_orders)
}

/// POST /api/staff/orders
pub async fn create_order(
    State(registry): State<AppRegistry>,
    Json(payload): Json<CreateOrderRequest>,
) -> (StatusCode, Json<Order>) {
    let new_order = registry.create_order(payload.items).await;
    (StatusCode::CREATED, Json(new_order))
}

/// POST /api/staff/production
pub async fn update_production(
    State(registry): State<AppRegistry>,
    Json(payload): Json<UpdateProductionRequest>,
) -> Json<UpdateProductionResponse> {
    let (newly_ready_orders, unallocated_items) = registry.update_production(payload.items).await;
    Json(UpdateProductionResponse {
        newly_ready_orders,
        unallocated_items,
    })
}

/// POST /api/staff/orders/{id}/complete
pub async fn complete_order(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<Order>, StatusCode> {
    let mut orders = registry.orders.lock().await;
    if let Some(order) = orders.iter_mut().find(|o| o.id == id) {
        order.status = OrderStatus::Completed;
        order.completed_at = Some(Utc::now());
        Ok(Json(order.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// POST /api/staff/orders/{id}/cancel
pub async fn cancel_order(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<Order>, StatusCode> {
    let mut orders = registry.orders.lock().await;
    if let Some(order) = orders.iter_mut().find(|o| o.id == id) {
        order.status = OrderStatus::Cancelled;
        // Note: a real implementation might need to handle stock implications of a cancellation.
        Ok(Json(order.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

//==// Bot API Handlers //==//

/// PUT /api/orders/{id}/notification
pub async fn add_notification(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
    Json(payload): Json<AddNotificationRequest>,
) -> Result<Json<Order>, StatusCode> {
    let mut orders = registry.orders.lock().await;
    if let Some(order) = orders.iter_mut().find(|o| o.id == id) {
        order.notify = Some(Notify {
            channel: payload.channel,
            target: payload.target,
        });
        // Optionally, send a confirmation notification
        // registry.send_notification(id, order.notify.as_ref().unwrap(), "Notification set successfully.".to_string()).await;
        Ok(Json(order.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

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
