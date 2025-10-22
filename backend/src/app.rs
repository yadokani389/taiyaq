use std::collections::HashMap;
use std::sync::Arc;

use bot_sdk_line::client::LINE;
use chrono::Utc;
use tokio::sync::Mutex;

use crate::api::model::AddNotificationRequest;
use crate::data::{Data, Item, Notify, Order, OrderStatus};

// AppRegistry is the main application state.
#[derive(Clone)]
pub struct AppRegistry {
    pub data: Arc<Mutex<Data>>,
    pub line: Arc<Mutex<LINE>>,
}

impl AppRegistry {
    const FILE_PATH: &str = "data.json";

    pub fn new(line_token: String) -> Self {
        Self {
            data: Arc::new(Mutex::new(Data::default())),
            line: Arc::new(Mutex::new(LINE::new(line_token))),
        }
    }

    pub async fn from_file(line_token: String) -> anyhow::Result<Self> {
        let registry = Self::new(line_token);
        registry.load_data().await?;
        Ok(registry)
    }

    pub async fn save_data(&self) -> anyhow::Result<()> {
        let data_str = serde_json::to_string_pretty(&*self.data.lock().await)?;
        std::fs::write(Self::FILE_PATH, data_str)?;
        Ok(())
    }

    pub async fn load_data(&self) -> anyhow::Result<()> {
        let data_str = std::fs::read_to_string(Self::FILE_PATH)?;
        let data: Data = serde_json::from_str(&data_str)?;
        *self.data.lock().await = data;
        Ok(())
    }

    // Atomically creates a new order and returns it.
    pub async fn create_order(&self, items: Vec<Item>) -> Order {
        let mut data = self.data.lock().await;
        let new_id = data.orders.iter().map(|o| o.id).max().unwrap_or(0) + 1;
        let new_order = Order {
            id: new_id,
            items,
            status: OrderStatus::Waiting, // All new orders start as 'waiting'
            ordered_at: Utc::now(),
            ready_at: None,
            completed_at: None,
            notify: None,
        };
        data.orders.push(new_order.clone());
        new_order
    }

    // Updates stock and fulfills waiting orders.
    pub async fn update_production(&self, production: Vec<Item>) -> (Vec<u32>, Vec<Item>) {
        let mut data = self.data.lock().await;

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
                if let Some(notify) = &order.notify {
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
        let mut data = self.data.lock().await;
        if let Some(order) = data.orders.iter_mut().find(|o| o.id == id) {
            order.status = OrderStatus::Completed;
            order.completed_at = Some(Utc::now());
            Some(order.clone())
        } else {
            None
        }
    }

    pub async fn cancel_order(&self, id: u32) -> Option<Order> {
        let mut data = self.data.lock().await;
        let Data {
            ref mut orders,
            ref mut unallocated_stock,
        } = *data;
        if let Some(order) = orders.iter_mut().find(|o| o.id == id) {
            // If the order was already ready, its items were deducted from unallocated_stock.
            // When cancelled, these items should be returned to unallocated_stock.
            if order.status == OrderStatus::Ready {
                for item in &order.items {
                    *unallocated_stock.entry(item.flavor.clone()).or_insert(0) += item.quantity;
                }
            }
            order.status = OrderStatus::Cancelled;
            Some(order.clone())
        } else {
            None
        }
    }

    pub async fn add_notification(
        &self,
        id: u32,
        payload: AddNotificationRequest,
    ) -> Option<Order> {
        let mut data = self.data.lock().await;
        if let Some(order) = data.orders.iter_mut().find(|o| o.id == id) {
            order.notify = Some(Notify {
                channel: payload.channel,
                target: payload.target,
            });
            Some(order.clone())
        } else {
            None
        }
    }

    pub async fn send_notification(&self, order_id: u32, notify: &Notify, message: String) {
        // TODO: This is a placeholder.
        println!(
            "Sending notification for Order ID: {}, Channel: {:?}, Target: {}, Message: {}",
            order_id, notify.channel, notify.target, message
        );
    }
}
