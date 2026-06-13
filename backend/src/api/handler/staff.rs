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
#[utoipa::path(
    get,
    path = "/api/staff/orders",
    tag = "staff",
    params(("status" = Option<String>, Query, description = "Comma-separated order statuses")),
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Staff order list", body = [StaffOrderResponse]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to load orders"),
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/staff/orders",
    tag = "staff",
    request_body = CreateOrderRequest,
    security(("staffBearerAuth" = [])),
    responses(
        (status = 201, description = "Created order", body = StaffOrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to save order"),
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/staff/stock",
    tag = "staff",
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Unallocated stock by flavor", body = Object),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to load stock"),
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/staff/production",
    tag = "staff",
    request_body = UpdateProductionRequest,
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Production update result", body = UpdateProductionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to save production update"),
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/staff/orders/{id}/complete",
    tag = "staff",
    params(("id" = u32, Path, description = "Order id")),
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Completed order", body = StaffOrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found"),
        (status = 500, description = "Failed to save completed order"),
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/staff/orders/{id}/cancel",
    tag = "staff",
    params(("id" = u32, Path, description = "Order id")),
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Cancelled order", body = StaffOrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found"),
        (status = 500, description = "Failed to save cancelled order"),
    )
)]
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
#[utoipa::path(
    put,
    path = "/api/staff/orders/{id}/priority",
    tag = "staff",
    params(("id" = u32, Path, description = "Order id")),
    request_body = UpdateOrderPriorityRequest,
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Updated order", body = StaffOrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found"),
        (status = 500, description = "Failed to save order priority update"),
    )
)]
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
#[utoipa::path(
    put,
    path = "/api/staff/orders/{id}/notification",
    tag = "staff",
    params(("id" = u32, Path, description = "Order id")),
    request_body = NotifyRequest,
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Updated order notification", body = StaffOrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found"),
        (status = 500, description = "Failed to save notification update"),
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/staff/flavors/config",
    tag = "staff",
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Flavor configs", body = Object),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to load flavor configs"),
    )
)]
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
#[utoipa::path(
    put,
    path = "/api/staff/flavors/{flavor}",
    tag = "staff",
    params(("flavor" = Flavor, Path, description = "Flavor")),
    request_body = FlavorConfig,
    security(("staffBearerAuth" = [])),
    responses(
        (status = 200, description = "Flavor config updated"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to save flavor config"),
    )
)]
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
