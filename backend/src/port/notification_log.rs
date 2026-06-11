use async_trait::async_trait;

use crate::domain::notification::NotificationDeliveryLog;
use crate::port::order_repository::OrderRepository;

#[async_trait]
pub trait NotificationLog: Send + Sync {
    async fn record_notification_delivery(
        &self,
        log: &NotificationDeliveryLog,
    ) -> anyhow::Result<()>;
}

pub trait AppRepository: OrderRepository + NotificationLog {}

impl<T> AppRepository for T where T: OrderRepository + NotificationLog {}
