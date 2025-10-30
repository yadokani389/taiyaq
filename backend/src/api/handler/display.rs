use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    api::model::{DisplayOrder, DisplayOrdersResponse, OrderDetailsResponse, WaitTimeResponse},
    app::AppRegistry,
    data::OrderStatus,
};

/// GET /api/orders/display
pub async fn get_display_orders(
    State(registry): State<AppRegistry>,
) -> Json<DisplayOrdersResponse> {
    let orders = &registry.data().await.orders;
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
    let waiting = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Waiting)
        .map(|o| DisplayOrder { id: o.id })
        .collect();
    Json(DisplayOrdersResponse {
        ready,
        cooking,
        waiting,
    })
}

/// GET /api/orders/{id}
pub async fn get_order_details(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<OrderDetailsResponse>, StatusCode> {
    if let Some(details) = registry.get_order_details(id).await {
        Ok(Json(details))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/wait-times
pub async fn get_wait_times(State(registry): State<AppRegistry>) -> Json<WaitTimeResponse> {
    Json(registry.get_current_wait_times().await)
}
