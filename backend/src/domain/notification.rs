use crate::domain::snapshot::Notify;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDeliveryLog {
    pub order_id: u32,
    pub target: Notify,
    pub message: String,
    pub status: NotificationDeliveryStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum NotificationDeliveryStatus {
    Sent,
    Failed,
}

impl NotificationDeliveryStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            NotificationDeliveryStatus::Sent => "sent",
            NotificationDeliveryStatus::Failed => "failed",
        }
    }
}
