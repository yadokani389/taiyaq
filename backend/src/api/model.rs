use crate::data::{Item, NotifyChannel, OrderStatus};
use serde::{Deserialize, Serialize};

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
pub struct AddNotificationRequest {
    pub channel: NotifyChannel,
    pub target: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrderPriorityRequest {
    pub is_priority: bool,
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

//==// Response Bodies //==//

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayOrdersResponse {
    pub ready: Vec<DisplayOrder>,
    pub cooking: Vec<DisplayOrder>,
    pub waiting: Vec<DisplayOrder>,
}

#[derive(Serialize)]
pub struct DisplayOrder {
    pub id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetailsResponse {
    pub id: u32,
    pub status: OrderStatus,
    pub estimated_wait_minutes: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProductionResponse {
    pub newly_ready_orders: Vec<u32>,
    pub unallocated_items: Vec<Item>,
}
