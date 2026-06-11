use chrono::{DateTime, Utc};
use enum_map::EnumMap;
use serde::{Deserialize, Serialize};

use crate::domain::order_number::DisplayOrderNumber;
use crate::domain::snapshot::{Flavor, Item, Notify, Order, OrderStatus};

//==// Request Bodies //==//

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderRequest {
    pub items: Vec<Item>,
    pub is_priority: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateProductionRequest {
    pub items: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrderPriorityRequest {
    pub is_priority: bool,
}

#[derive(Deserialize)]
pub enum NotifyRequest {
    Discord { channel_id: u64, user_id: u64 },
    Line { user_id: String },
}

impl From<NotifyRequest> for Notify {
    fn from(request: NotifyRequest) -> Self {
        match request {
            NotifyRequest::Discord {
                channel_id,
                user_id,
            } => Notify::Discord {
                channel_id,
                user_id,
            },
            NotifyRequest::Line { user_id } => Notify::Line { user_id },
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct StaffOrdersQuery {
    #[serde(default, deserialize_with = "csv_to_order_status")]
    pub status: Vec<OrderStatus>,
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
        .map(|s_trim| OrderStatus::from_api_str(s_trim.trim()).map_err(serde::de::Error::custom))
        .collect()
}

//==// Response Bodies //==//

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayOrdersResponse {
    pub ready: Vec<DisplayOrder>,
    pub cooking: Vec<DisplayOrder>,
    pub waiting: Vec<DisplayOrder>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayOrder {
    pub id: u32,
    pub display_number: String,
}

impl DisplayOrder {
    pub fn from_id(id: u32) -> Self {
        Self {
            id,
            display_number: DisplayOrderNumber::from_order_id(id).as_str(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetailsResponse {
    pub id: u32,
    pub display_number: String,
    pub items: Vec<Item>,
    pub status: OrderStatus,
    pub ordered_at: DateTime<Utc>,
    pub estimated_wait_minutes: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitTimeResponse {
    pub wait_times: EnumMap<Flavor, Option<i64>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProductionResponse {
    pub newly_ready_orders: Vec<u32>,
    pub unallocated_items: Vec<Item>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffOrderResponse {
    pub id: u32,
    pub display_number: String,
    pub items: Vec<Item>,
    pub status: OrderStatus,
    pub ordered_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub is_priority: bool,
}

impl From<Order> for StaffOrderResponse {
    fn from(order: Order) -> Self {
        Self {
            display_number: DisplayOrderNumber::from_order_id(order.id).as_str(),
            id: order.id,
            items: order.items,
            status: order.status,
            ordered_at: order.ordered_at,
            ready_at: order.ready_at,
            completed_at: order.completed_at,
            is_priority: order.is_priority,
        }
    }
}
