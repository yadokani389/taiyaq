use bot_sdk_line::client::LINE;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type AppRegistry = Arc<Data>;

// Main data structure holding the application state.
pub struct Data {
    pub orders: Mutex<Vec<Order>>,
    // In-memory stock of baked taiyaki not yet allocated to an order.
    pub unallocated_stock: Mutex<HashMap<String, usize>>,
    pub line: Mutex<LINE>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: u32,
    pub items: Vec<Item>,
    pub status: OrderStatus,
    pub ordered_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notify: Option<Notify>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub flavor: String,
    pub quantity: usize,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, Copy)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    #[default]
    Waiting,
    Cooking,
    Ready,
    Completed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Notify {
    pub channel: NotifyChannel,
    pub target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotifyChannel {
    Discord,
    Email,
    Line,
}

impl Data {
    pub fn new(line_token: String) -> Self {
        Self {
            orders: Mutex::new(Vec::new()),
            unallocated_stock: Mutex::new(HashMap::new()),
            line: Mutex::new(LINE::new(line_token)),
        }
    }

    // Atomically creates a new order and returns it.
    pub async fn create_order(&self, items: Vec<Item>) -> Order {
        let mut orders = self.orders.lock().await;
        let new_id = orders.iter().map(|o| o.id).max().unwrap_or(0) + 1;
        let new_order = Order {
            id: new_id,
            items,
            status: OrderStatus::Waiting, // All new orders start as 'waiting'
            ordered_at: Utc::now(),
            ready_at: None,
            completed_at: None,
            notify: None,
        };
        orders.push(new_order.clone());
        new_order
    }

    // Updates stock and fulfills waiting orders.
    pub async fn update_production(&self, production: Vec<Item>) -> (Vec<u32>, Vec<Item>) {
        let mut stock = self.unallocated_stock.lock().await;
        let mut orders = self.orders.lock().await;

        // 1. Add new production to stock
        for item in production {
            *stock.entry(item.flavor).or_insert(0) += item.quantity;
        }

        let mut newly_ready_orders = Vec::new();

        // 2. Iterate through waiting orders (FIFO) and try to fulfill them
        for order in orders
            .iter_mut()
            .filter(|o| o.status == OrderStatus::Waiting)
        {
            if Self::can_fulfill(order, &stock) {
                Self::fulfill(order, &mut stock);
                order.status = OrderStatus::Ready;
                order.ready_at = Some(Utc::now());
                newly_ready_orders.push(order.id);
                // TODO: Trigger notification if `order.notify` is Some.
            }
        }

        let unallocated_items = stock
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

    // Add a placeholder for sending notifications
    #[allow(unused)]
    pub async fn send_notification(&self, order_id: u32, notify: &Notify, message: String) {
        // This is a placeholder. Actual notification sending logic would go here.
        println!(
            "Sending notification for Order ID: {}, Channel: {:?}, Target: {}, Message: {}",
            order_id, notify.channel, notify.target, message
        );
        // For LINE, you would use self.line.lock().await.messaging_api_client.push_message(...)
        // For Discord, you would use a Discord webhook or API.
        // For Email, you would use an email sending library.
    }
}
