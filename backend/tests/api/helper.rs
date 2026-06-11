use std::sync::Arc;

use async_trait::async_trait;
use axum::{Router, body, http::request::Builder};
use bot_sdk_line::messaging_api_line::models::Message;
use chrono::{TimeZone, Utc};
use tokio::sync::Mutex;

use taiyaq_backend::api::route::routes;
use taiyaq_backend::app::AppRegistry;
use taiyaq_backend::domain::notification::NotificationDeliveryLog;
use taiyaq_backend::domain::snapshot::{Item, Notify, Order, OrderStatus, OrderSystemSnapshot};
use taiyaq_backend::port::line_reply::LineReplySender;
use taiyaq_backend::port::notification_log::NotificationLog;
use taiyaq_backend::port::notifier::Notifier;
use taiyaq_backend::port::order_repository::OrderRepository;

pub fn make_router(registry: AppRegistry) -> Router {
    routes(registry)
}

pub fn registry_with_snapshot(arrange: impl FnOnce(&mut OrderSystemSnapshot)) -> AppRegistry {
    let mut snapshot = OrderSystemSnapshot::default();
    arrange(&mut snapshot);
    let repository = Arc::new(FakeRepository::new(snapshot));
    let notifier = Arc::new(FakeNotifier);
    AppRegistry::new_with_ports(
        repository,
        notifier.clone(),
        notifier,
        "test-token".to_owned(),
        "test-line-secret".to_owned(),
    )
}

pub fn order(id: u32, status: OrderStatus) -> Order {
    Order {
        id,
        items: Vec::new(),
        status,
        ordered_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, id).unwrap(),
        ready_at: None,
        completed_at: None,
        notify: Default::default(),
        is_priority: false,
    }
}

pub fn waiting_order_with_items(id: u32, items: Vec<Item>) -> Order {
    Order {
        items,
        ..order(id, OrderStatus::Waiting)
    }
}

pub async fn deserialize_json(
    response: axum::response::Response,
) -> anyhow::Result<serde_json::Value> {
    let bytes = body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub trait TestRequestExt {
    fn bearer(self) -> Builder;
    fn application_json(self) -> Builder;
}

impl TestRequestExt for Builder {
    fn bearer(self) -> Builder {
        self.header("Authorization", "Bearer test-token")
    }

    fn application_json(self) -> Builder {
        self.header("Content-Type", "application/json")
    }
}

struct FakeRepository {
    snapshot: Mutex<OrderSystemSnapshot>,
    notification_logs: Mutex<Vec<NotificationDeliveryLog>>,
}

impl FakeRepository {
    fn new(snapshot: OrderSystemSnapshot) -> Self {
        Self {
            snapshot: Mutex::new(snapshot),
            notification_logs: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl OrderRepository for FakeRepository {
    async fn load_snapshot(&self) -> anyhow::Result<OrderSystemSnapshot> {
        Ok(self.snapshot.lock().await.clone())
    }

    async fn replace_snapshot(&self, snapshot: &OrderSystemSnapshot) -> anyhow::Result<()> {
        *self.snapshot.lock().await = snapshot.clone();
        Ok(())
    }
}

#[async_trait]
impl NotificationLog for FakeRepository {
    async fn record_notification_delivery(
        &self,
        log: &NotificationDeliveryLog,
    ) -> anyhow::Result<()> {
        self.notification_logs.lock().await.push(log.clone());
        Ok(())
    }
}

struct FakeNotifier;

#[async_trait]
impl Notifier for FakeNotifier {
    async fn send(&self, _target: Notify, _message: String) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl LineReplySender for FakeNotifier {
    async fn reply(&self, _reply_token: String, _messages: Vec<Message>) -> anyhow::Result<()> {
        Ok(())
    }
}
