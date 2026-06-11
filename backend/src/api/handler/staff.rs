use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use enum_map::EnumMap;
use tracing::{error, info};

use crate::{
    api::model::{
        CreateOrderRequest, NotifyRequest, StaffOrderResponse, StaffOrdersQuery,
        UpdateOrderPriorityRequest, UpdateProductionRequest, UpdateProductionResponse,
    },
    app::AppRegistry,
    domain::snapshot::{Flavor, FlavorConfig},
};
/// GET /api/staff/orders
pub async fn get_staff_orders(
    State(registry): State<AppRegistry>,
    Query(query): Query<StaffOrdersQuery>,
) -> Result<Json<Vec<StaffOrderResponse>>, StatusCode> {
    let snapshot = registry.snapshot().await.map_err(|error| {
        error!(?error, "failed to load orders");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let orders = &snapshot.orders;
    let filtered_orders = if query.status.is_empty() {
        orders.clone()
    } else {
        orders
            .iter()
            .filter(|o| query.status.contains(&o.status))
            .cloned()
            .collect()
    };
    Ok(Json(
        filtered_orders
            .into_iter()
            .map(StaffOrderResponse::from)
            .collect(),
    ))
}

/// POST /api/staff/orders
pub async fn create_order(
    State(registry): State<AppRegistry>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<StaffOrderResponse>), StatusCode> {
    info!(items = ?payload.items, "creating order");
    let new_order = registry
        .create_order(payload.items, payload.is_priority.unwrap_or(false))
        .await
        .map_err(|error| {
            error!(?error, "failed to save order");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok((StatusCode::CREATED, Json(new_order.into())))
}

/// GET /api/staff/stock
pub async fn get_stock(
    State(registry): State<AppRegistry>,
) -> Result<Json<EnumMap<Flavor, usize>>, StatusCode> {
    let snapshot = registry.snapshot().await.map_err(|error| {
        error!(?error, "failed to load stock");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(snapshot.unallocated_stock))
}

/// POST /api/staff/production
pub async fn update_production(
    State(registry): State<AppRegistry>,
    Json(payload): Json<UpdateProductionRequest>,
) -> Result<Json<UpdateProductionResponse>, StatusCode> {
    let (newly_ready_orders, unallocated_items) = registry
        .update_production(payload.items)
        .await
        .map_err(|error| {
        error!(?error, "failed to save production update");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(UpdateProductionResponse {
        newly_ready_orders,
        unallocated_items,
    }))
}

/// POST /api/staff/orders/{id}/complete
pub async fn complete_order(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<StaffOrderResponse>, StatusCode> {
    if let Some(order) = registry.complete_order(id).await.map_err(|error| {
        error!(?error, order_id = id, "failed to save completed order");
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
        Ok(Json(order.into()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// POST /api/staff/orders/{id}/cancel
pub async fn cancel_order(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
) -> Result<Json<StaffOrderResponse>, StatusCode> {
    if let Some(order) = registry.cancel_order(id).await.map_err(|error| {
        error!(?error, order_id = id, "failed to save cancelled order");
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
        Ok(Json(order.into()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// PUT /api/staff/orders/{id}/priority
pub async fn update_order_priority(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
    Json(payload): Json<UpdateOrderPriorityRequest>,
) -> Result<Json<StaffOrderResponse>, StatusCode> {
    if let Some(order) = registry
        .update_order_priority(id, payload.is_priority)
        .await
        .map_err(|error| {
            error!(
                ?error,
                order_id = id,
                "failed to save order priority update"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    {
        Ok(Json(order.into()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// PUT /api/orders/{id}/notification
pub async fn add_notification(
    State(registry): State<AppRegistry>,
    Path(id): Path<u32>,
    Json(payload): Json<NotifyRequest>,
) -> Result<Json<StaffOrderResponse>, StatusCode> {
    if let Some(order) = registry
        .add_notification(id, payload.into())
        .await
        .map_err(|error| {
            error!(?error, order_id = id, "failed to save notification update");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    {
        Ok(Json(order.into()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
/// GET /api/staff/flavors/config
pub async fn get_flavor_configs(
    State(registry): State<AppRegistry>,
) -> Result<Json<EnumMap<Flavor, FlavorConfig>>, StatusCode> {
    let snapshot = registry.snapshot().await.map_err(|error| {
        error!(?error, "failed to load flavor configs");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(snapshot.flavor_configs))
}

/// PUT /api/staff/flavors/{flavor}
pub async fn set_flavor_config(
    State(registry): State<AppRegistry>,
    Path(flavor): Path<Flavor>,
    Json(config): Json<FlavorConfig>,
) -> Result<StatusCode, StatusCode> {
    registry
        .set_flavor_config(flavor, config)
        .await
        .map_err(|error| {
            error!(?error, ?flavor, "failed to save flavor config");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(StatusCode::OK)
}
