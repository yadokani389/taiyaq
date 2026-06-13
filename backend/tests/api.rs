#[path = "api/helper.rs"]
mod helper;

use axum::{
    body::{self, Body},
    http::Request,
};
use taiyaq_backend::domain::snapshot::{Flavor, Item, OrderStatus};
use tower::ServiceExt;

use crate::helper::{TestRequestExt, deserialize_json, make_router, registry_with_snapshot};

#[tokio::test]
async fn get_openapi_json_200_includes_paths() -> anyhow::Result<()> {
    let app = make_router(registry_with_snapshot(|_| {}));

    let response = app
        .oneshot(Request::get("/openapi.json").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = deserialize_json(response).await?;
    assert_eq!(body["openapi"], "3.1.0");
    assert!(body["paths"]["/api/orders/display"].is_object());
    assert!(body["paths"]["/api/staff/orders"].is_object());
    Ok(())
}

#[tokio::test]
async fn get_swagger_ui_200_returns_html() -> anyhow::Result<()> {
    let app = make_router(registry_with_snapshot(|_| {}));

    let response = app
        .oneshot(Request::get("/swagger-ui/").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let bytes = body::to_bytes(response.into_body(), usize::MAX).await?;
    let body = std::str::from_utf8(&bytes)?;
    assert!(body.contains("Swagger UI"));
    Ok(())
}

#[tokio::test]
async fn get_display_orders_200_groups_orders_by_status() -> anyhow::Result<()> {
    let app = make_router(registry_with_snapshot(|snapshot| {
        snapshot.orders = vec![
            helper::order(1, OrderStatus::Ready),
            helper::order(2, OrderStatus::Cooking),
            helper::order(3, OrderStatus::Waiting),
            helper::order(4, OrderStatus::Completed),
        ];
    }));

    let response = app
        .oneshot(Request::get("/api/orders/display").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = deserialize_json(response).await?;
    assert_eq!(body["ready"][0]["id"], 1);
    assert_eq!(body["ready"][0]["displayNumber"], "01");
    assert_eq!(body["cooking"][0]["id"], 2);
    assert_eq!(body["waiting"][0]["id"], 3);
    assert_eq!(
        body["ready"].as_array().expect("ready is an array").len(),
        1
    );

    Ok(())
}

#[tokio::test]
async fn get_order_details_404_for_unknown_order() -> anyhow::Result<()> {
    let app = make_router(registry_with_snapshot(|snapshot| {
        snapshot.orders = vec![helper::order(1, OrderStatus::Waiting)];
    }));

    let response = app
        .oneshot(Request::get("/api/orders/999").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn create_staff_order_401_without_bearer_token() -> anyhow::Result<()> {
    let app = make_router(registry_with_snapshot(|_| {}));

    let request = serde_json::json!({
        "items": [{ "flavor": "tsubuan", "quantity": 1 }]
    });
    let response = app
        .oneshot(
            Request::post("/api/staff/orders")
                .application_json()
                .body(Body::from(request.to_string()))?,
        )
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn create_staff_order_201_creates_order() -> anyhow::Result<()> {
    let registry = registry_with_snapshot(|snapshot| {
        snapshot.unallocated_stock[Flavor::Tsubuan] = 1;
    });
    let app = make_router(registry.clone());

    let request = serde_json::json!({
        "items": [{ "flavor": "tsubuan", "quantity": 1 }],
        "isPriority": true
    });
    let response = app
        .oneshot(
            Request::post("/api/staff/orders")
                .bearer()
                .application_json()
                .body(Body::from(request.to_string()))?,
        )
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
    let body = deserialize_json(response).await?;
    assert_eq!(body["id"], 1);
    assert_eq!(body["status"], "ready");
    assert_eq!(body["isPriority"], true);

    let snapshot = registry.snapshot().await?;
    assert_eq!(snapshot.orders.len(), 1);
    assert_eq!(snapshot.unallocated_stock[Flavor::Tsubuan], 0);
    Ok(())
}

#[tokio::test]
async fn update_production_200_returns_newly_ready_orders() -> anyhow::Result<()> {
    let app = make_router(registry_with_snapshot(|snapshot| {
        snapshot.orders = vec![helper::waiting_order_with_items(
            1,
            vec![Item {
                flavor: Flavor::Tsubuan,
                quantity: 1,
            }],
        )];
    }));

    let request = serde_json::json!({
        "items": [{ "flavor": "tsubuan", "quantity": 1 }]
    });
    let response = app
        .oneshot(
            Request::post("/api/staff/production")
                .bearer()
                .application_json()
                .body(Body::from(request.to_string()))?,
        )
        .await?;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = deserialize_json(response).await?;
    assert_eq!(body["newlyReadyOrders"], serde_json::json!([1]));
    assert_eq!(body["unallocatedItems"], serde_json::json!([]));
    Ok(())
}
