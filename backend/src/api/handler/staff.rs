use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use enum_map::EnumMap;

use crate::{
    api::model::{
        AddNotificationRequest, CreateOrderRequest, StaffOrdersQuery, UpdateOrderPriorityRequest,
        UpdateProductionRequest, UpdateProductionResponse,
    },
    app::AppRegistry,
    data::{Flavor, FlavorConfig, Order},
};

/// GET /api/staff/orders
pub async fn get_staff_orders(
    State(registry): State<AppRegistry>,
    Query(query): Query<StaffOrdersQuery>,
) -> Json<Vec<Order>> {
    let orders = &registry.data().await.orders;
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
    println!("Creating order with items: {:?}", payload.items);
    let new_order = registry
        .create_order(payload.items, payload.is_priority.unwrap_or(false))
        .await;
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

/// PUT /api/staff/orders/{id}/priority
pub async fn update_order_priority(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
    Json(payload): Json<UpdateOrderPriorityRequest>,
) -> Result<Json<Order>, StatusCode> {
    if let Some(order) = registry
        .update_order_priority(id, payload.is_priority)
        .await
    {
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

/// GET /api/staff/flavors/config
pub async fn get_flavor_configs(
    State(registry): State<AppRegistry>,
) -> Json<EnumMap<Flavor, FlavorConfig>> {
    let data = registry.data().await;
    Json(data.flavor_configs.clone())
}

/// PUT /api/staff/flavors/{flavor}
pub async fn set_flavor_config(
    State(registry): State<AppRegistry>,
    Path(flavor): Path<Flavor>,
    Json(config): Json<FlavorConfig>,
) -> StatusCode {
    registry.set_flavor_config(flavor, config).await;
    StatusCode::OK
}
