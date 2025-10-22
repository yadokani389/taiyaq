use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use crate::{
    api::model::{
        AddNotificationRequest, CreateOrderRequest, StaffOrdersQuery, UpdateProductionRequest,
        UpdateProductionResponse,
    },
    app::AppRegistry,
    data::Order,
};

/// GET /api/staff/orders
pub async fn get_staff_orders(
    State(registry): State<AppRegistry>,
    Query(query): Query<StaffOrdersQuery>,
) -> Json<Vec<Order>> {
    let data = registry.data.lock().await;
    let orders = &data.orders;
    let filtered_orders = if query.status.is_empty() {
        orders.clone()
    } else {
        orders
            .iter()
            .filter(|o| query.status.contains(&o.status))
            .cloned()
            .collect()
    };
    Json(filtered_orders)
}

/// POST /api/staff/orders
pub async fn create_order(
    State(registry): State<AppRegistry>,
    Json(payload): Json<CreateOrderRequest>,
) -> (StatusCode, Json<Order>) {
    let new_order = registry.create_order(payload.items).await;
    (StatusCode::CREATED, Json(new_order))
}

/// POST /api/staff/production
pub async fn update_production(
    State(registry): State<AppRegistry>,
    Json(payload): Json<UpdateProductionRequest>,
) -> Json<UpdateProductionResponse> {
    let (newly_ready_orders, unallocated_items) = registry.update_production(payload.items).await;
    Json(UpdateProductionResponse {
        newly_ready_orders,
        unallocated_items,
    })
}

/// POST /api/staff/orders/{id}/complete
pub async fn complete_order(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<Order>, StatusCode> {
    if let Some(order) = registry.complete_order(id).await {
        Ok(Json(order))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// POST /api/staff/orders/{id}/cancel
pub async fn cancel_order(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<Order>, StatusCode> {
    if let Some(order) = registry.cancel_order(id).await {
        Ok(Json(order))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// PUT /api/orders/{id}/notification
pub async fn add_notification(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
    Json(payload): Json<AddNotificationRequest>,
) -> Result<Json<Order>, StatusCode> {
    if let Some(order) = registry.add_notification(id, payload).await {
        Ok(Json(order))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
