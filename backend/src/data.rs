use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Data holds the core business data.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Data {
    pub orders: Vec<Order>,
    pub unallocated_stock: HashMap<String, usize>,
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
