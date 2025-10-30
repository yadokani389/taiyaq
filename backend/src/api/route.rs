use crate::{
    api::{
        auth::staff_api_auth,
        handler::{
            add_notification, cancel_order, complete_order, create_order, get_display_orders,
            get_order_details, get_staff_orders, line_callback, set_flavor_config,
            update_order_priority, update_production,
        },
    },
    app::AppRegistry,
};
use axum::{
    Router, middleware,
    routing::{get, post, put},
};

pub fn routes() -> Router<AppRegistry> {
    let user_routes = Router::new()
        .route("/orders/display", get(get_display_orders))
        .route("/orders/{id}", get(get_order_details));

    let staff_routes = Router::new()
        .route("/staff/orders", get(get_staff_orders).post(create_order))
        .route("/staff/production", post(update_production))
        .route("/staff/orders/{id}/complete", post(complete_order))
        .route("/staff/orders/{id}/cancel", post(cancel_order))
        .route("/staff/orders/{id}/priority", put(update_order_priority))
        .route("/staff/orders/{id}/notification", put(add_notification))
        .route("/staff/flavors/{flavor}", put(set_flavor_config))
        .layer(middleware::from_fn(staff_api_auth));

    let line_router = Router::new().route("/line_callback", post(line_callback));

    Router::new()
        .nest("/api", user_routes.merge(staff_routes))
        .merge(line_router)
}
