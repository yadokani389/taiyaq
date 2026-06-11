use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::error;

use crate::{
    api::model::{DisplayOrder, DisplayOrdersResponse, OrderDetailsResponse, WaitTimeResponse},
    app::AppRegistry,
    domain::snapshot::OrderStatus,
};

/// GET /api/orders/display
pub async fn get_display_orders(
    State(registry): State<AppRegistry>,
) -> Result<Json<DisplayOrdersResponse>, StatusCode> {
    let snapshot = registry.snapshot().await.map_err(|error| {
        error!(?error, "failed to load display orders");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let orders = &snapshot.orders;
    let ready = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Ready)
        .map(|o| DisplayOrder::from_id(o.id))
        .collect();
    let cooking = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Cooking)
        .map(|o| DisplayOrder::from_id(o.id))
        .collect();
    let waiting = orders
        .iter()
        .filter(|o| o.status == OrderStatus::Waiting)
        .map(|o| DisplayOrder::from_id(o.id))
        .collect();
    Ok(Json(DisplayOrdersResponse {
        ready,
        cooking,
        waiting,
    }))
}

/// GET /api/orders/{id}
pub async fn get_order_details(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<OrderDetailsResponse>, StatusCode> {
    if let Some(details) = registry.get_order_details(id).await.map_err(|error| {
        error!(?error, order_id = id, "failed to load order details");
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
        Ok(Json(details))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/wait-times
pub async fn get_wait_times(
    State(registry): State<AppRegistry>,
) -> Result<Json<WaitTimeResponse>, StatusCode> {
    registry
        .get_current_wait_times()
        .await
        .map(Json)
        .map_err(|error| {
            error!(?error, "failed to load wait times");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
