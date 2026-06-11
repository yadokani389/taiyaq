use async_trait::async_trait;

use crate::domain::snapshot::Notify;

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, target: Notify, message: String) -> anyhow::Result<()>;
}
