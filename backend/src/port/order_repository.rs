use async_trait::async_trait;

use crate::domain::snapshot::OrderSystemSnapshot;

#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn load_snapshot(&self) -> anyhow::Result<OrderSystemSnapshot>;
    async fn replace_snapshot(&self, snapshot: &OrderSystemSnapshot) -> anyhow::Result<()>;
}
