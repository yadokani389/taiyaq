use crate::data::{Item, NotifyChannel, OrderStatus};
use serde::{Deserialize, Serialize};

//==// Request Bodies //==//

#[derive(Deserialize)]
pub struct CreateOrderRequest {
    pub items: Vec<Item>,
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

//==// Response Bodies //==//

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayOrdersResponse {
    pub ready: Vec<DisplayOrder>,
    pub cooking: Vec<DisplayOrder>,
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
