use axum::Json;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{
    api::{
        handler::{
            __path_add_notification, __path_cancel_order, __path_complete_order,
            __path_create_order, __path_get_display_orders, __path_get_flavor_configs,
            __path_get_order_details, __path_get_staff_orders, __path_get_stock,
            __path_get_wait_times, __path_line_callback, __path_set_flavor_config,
            __path_update_order_priority, __path_update_production,
        },
        model::{
            CreateOrderRequest, DisplayOrder, DisplayOrdersResponse, NotifyRequest,
            OrderDetailsResponse, StaffOrderResponse, UpdateOrderPriorityRequest,
            UpdateProductionRequest, UpdateProductionResponse, WaitTimeResponse,
        },
    },
    domain::snapshot::{Flavor, FlavorConfig, Item, Notify, OrderStatus},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_display_orders,
        get_order_details,
        get_wait_times,
        get_staff_orders,
        create_order,
        get_stock,
        update_production,
        complete_order,
        cancel_order,
        update_order_priority,
        add_notification,
        get_flavor_configs,
        set_flavor_config,
        line_callback,
    ),
    components(schemas(
        CreateOrderRequest,
        DisplayOrder,
        DisplayOrdersResponse,
        Flavor,
        FlavorConfig,
        Item,
        Notify,
        NotifyRequest,
        OrderDetailsResponse,
        OrderStatus,
        StaffOrderResponse,
        UpdateOrderPriorityRequest,
        UpdateProductionRequest,
        UpdateProductionResponse,
        WaitTimeResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "display", description = "Public order display APIs"),
        (name = "staff", description = "Staff operation APIs"),
        (name = "line", description = "LINE webhook APIs"),
    )
)]
struct TaiyaqOpenApi;

pub fn build_openapi() -> utoipa::openapi::OpenApi {
    TaiyaqOpenApi::openapi()
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "staffBearerAuth",
                SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
            );
        }
    }
}

pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(build_openapi())
}
