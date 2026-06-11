use async_trait::async_trait;
use bot_sdk_line::messaging_api_line::models::Message;

#[async_trait]
pub trait LineReplySender: Send + Sync {
    async fn reply(&self, reply_token: String, messages: Vec<Message>) -> anyhow::Result<()>;
}
