use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    api::model::{DisplayOrder, DisplayOrdersResponse, OrderDetailsResponse},
    app::AppRegistry,
    data::OrderStatus,
};

/// GET /api/orders/display
pub async fn get_display_orders(
    State(registry): State<AppRegistry>,
) -> Json<DisplayOrdersResponse> {
    let data_guard = registry.data.lock().await;
    let orders = &data_guard.orders;
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
    let data_guard = registry.data.lock().await;
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
        Ok(Json(OrderDetailsResponse {
            id: order.id,
            status: order.status,
            estimated_wait_minutes,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
