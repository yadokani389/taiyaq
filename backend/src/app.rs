use std::collections::HashMap;
use std::sync::Arc;

use bot_sdk_line::client::LINE;
use bot_sdk_line::messaging_api_line::{
    apis::MessagingApiApi,
    models::{Message, PushMessageRequest, TextMessageV2},
};
use chrono::Utc;
use poise::serenity_prelude::Context;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

use crate::api::model::{AddNotificationRequest, OrderDetailsResponse};
use crate::data::{Data, Item, Notify, NotifyChannel, Order, OrderStatus};
use crate::discord;

// AppRegistry is the main application state.
#[derive(Clone)]
pub struct AppRegistry {
    data: Arc<RwLock<Data>>,
    pub line: Arc<Mutex<LINE>>,
    pub discord_ctx: Arc<Mutex<Context>>,
}

impl AppRegistry {
    const FILE_PATH: &str = "data.json";

    pub fn new(line_token: String, ctx: Context) -> Self {
        Self {
            data: Arc::new(RwLock::new(Data::default())),
            line: Arc::new(Mutex::new(LINE::new(line_token))),
            discord_ctx: Arc::new(Mutex::new(ctx)),
        }
    }

    pub async fn save_data(&self) -> anyhow::Result<()> {
        let data_str = serde_json::to_string_pretty(&*self.data.read().await)?;
        std::fs::write(Self::FILE_PATH, data_str)?;
        Ok(())
    }

    pub async fn load_data(&self) -> anyhow::Result<()> {
        let data_str = std::fs::read_to_string(Self::FILE_PATH)?;
        let data: Data = serde_json::from_str(&data_str)?;
        *self.data.write().await = data;
        Ok(())
    }

    pub async fn data(&self) -> RwLockReadGuard<'_, Data> {
        self.data.read().await
    }

    // Atomically creates a new order and returns it.
    pub async fn create_order(&self, items: Vec<Item>) -> Order {
        let mut data = self.data.write().await;
        let new_id = data.orders.iter().map(|o| o.id).max().unwrap_or(0) + 1;
        let new_order = Order {
            id: new_id,
            items,
            status: OrderStatus::Waiting, // All new orders start as 'waiting'
            ordered_at: Utc::now(),
            ready_at: None,
            completed_at: None,
            notify: vec![],
        };
        data.orders.push(new_order.clone());
        drop(data);
        self.save_data().await.ok();
        new_order
    }

    // Updates stock and fulfills waiting orders.
    pub async fn update_production(&self, production: Vec<Item>) -> (Vec<u32>, Vec<Item>) {
        let mut data = self.data.write().await;

        for item in production {
            *data.unallocated_stock.entry(item.flavor).or_insert(0) += item.quantity;
        }

        let mut newly_ready_orders = Vec::new();
        let mut temp_unallocated_stock = std::mem::take(&mut data.unallocated_stock);

        for order in data
            .orders
            .iter_mut()
            .filter(|o| o.status == OrderStatus::Waiting)
        {
            if Self::can_fulfill(order, &temp_unallocated_stock) {
                Self::fulfill(order, &mut temp_unallocated_stock);
                order.status = OrderStatus::Ready;
                order.ready_at = Some(Utc::now());
                newly_ready_orders.push(order.id);
                for notify in &order.notify {
                    self.send_notification(
                        order.id,
                        notify,
                        format!("Your order #{} is ready for pickup!", order.id),
                    )
                    .await;
                }
            }
        }
        data.unallocated_stock = temp_unallocated_stock;

        let unallocated_items = data
            .unallocated_stock
            .iter()
            .filter(|&(_, &quantity)| quantity > 0)
            .map(|(flavor, &quantity)| Item {
                flavor: flavor.clone(),
                quantity,
            })
            .collect();

        drop(data);
        self.save_data().await.ok();
        (newly_ready_orders, unallocated_items)
    }

    // Helper to check if an order can be fulfilled from stock
    fn can_fulfill(order: &Order, stock: &HashMap<String, usize>) -> bool {
        order
            .items
            .iter()
            .all(|item| stock.get(&item.flavor).unwrap_or(&0) >= &item.quantity)
    }

    // Helper to decrement stock for a fulfilled order
    fn fulfill(order: &Order, stock: &mut HashMap<String, usize>) {
        for item in &order.items {
            if let Some(stock_qty) = stock.get_mut(&item.flavor) {
                *stock_qty -= item.quantity;
            }
        }
    }

    pub async fn complete_order(&self, id: u32) -> Option<Order> {
        let mut data = self.data.write().await;
        let order = data.orders.iter_mut().find(|o| o.id == id)?;
        order.status = OrderStatus::Completed;
        order.completed_at = Some(Utc::now());

        let order = order.clone();
        drop(data);
        self.save_data().await.ok();
        Some(order)
    }

    pub async fn cancel_order(&self, id: u32) -> Option<Order> {
        let mut data = self.data.write().await;
        let Data {
            ref mut orders,
            ref mut unallocated_stock,
        } = *data;
        let order = orders.iter_mut().find(|o| o.id == id)?;
        // If the order was already ready, its items were deducted from unallocated_stock.
        // When cancelled, these items should be returned to unallocated_stock.
        if order.status == OrderStatus::Ready {
            for item in &order.items {
                *unallocated_stock.entry(item.flavor.clone()).or_insert(0) += item.quantity;
            }
        }
        order.status = OrderStatus::Cancelled;

        let order = order.clone();
        drop(data);
        self.save_data().await.ok();
        Some(order)
    }

    pub async fn add_notification(
        &self,
        id: u32,
        payload: AddNotificationRequest,
    ) -> Option<Order> {
        let mut data = self.data.write().await;
        let order = data.orders.iter_mut().find(|o| o.id == id)?;
        order.notify.push(Notify {
            channel: payload.channel,
            target: payload.target,
        });
        let order = order.clone();
        drop(data);
        self.save_data().await.ok();
        Some(order)
    }

    pub async fn send_notification(&self, order_id: u32, notify: &Notify, message: String) {
        // TODO: This is a placeholder.
        println!(
            "Sending notification for Order ID: {}, Channel: {:?}, Target: {}, Message: {}",
            order_id, notify.channel, notify.target, message
        );

        match notify.channel {
            NotifyChannel::Discord => {
                let ctx = self.discord_ctx.lock().await;
                let user_id: u64 = notify.target.parse().unwrap_or(0);
                if user_id != 0 {
                    discord::send_dm(&ctx, user_id, &message).await.ok();
                }
            }
            NotifyChannel::Email => todo!(),
            NotifyChannel::Line => {
                let line = self.line.lock().await;
                let push_request = PushMessageRequest {
                    to: notify.target.clone(), // LINE user_id
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
                        println!("✅ LINE notification sent to user {}", notify.target);
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to send LINE notification to {}: {:?}",
                            notify.target, e
                        );
                    }
                }
            }
        }
    }

    pub async fn get_order_details(&self, id: u32) -> Option<OrderDetailsResponse> {
        let data_guard = self.data.read().await;
        let orders = &data_guard.orders;
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
            Some(OrderDetailsResponse {
                id: order.id,
                status: order.status,
                estimated_wait_minutes,
            })
        } else {
            None
        }
    }
}
