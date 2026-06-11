use std::sync::Arc;

use async_trait::async_trait;
use bot_sdk_line::client::LINE;
use bot_sdk_line::messaging_api_line::apis::MessagingApiApi;
use bot_sdk_line::messaging_api_line::models::{Message, ReplyMessageRequest};
use poise::serenity_prelude::Context;
use tokio::sync::Mutex;
use tracing::error;

use crate::api::model::{OrderDetailsResponse, WaitTimeResponse};
use crate::domain::notification::{NotificationDeliveryLog, NotificationDeliveryStatus};
use crate::domain::order_number::DisplayOrderNumber;
use crate::domain::order_status;
use crate::domain::snapshot::{Flavor, FlavorConfig, Item, Notify, Order, OrderSystemSnapshot};
use crate::domain::wait_time;
use crate::port::line_reply::LineReplySender;
use crate::port::notification_log::AppRepository;
use crate::port::notifier::Notifier;
use crate::storage::SqliteRepository;
use crate::usecase::order;
use crate::usecase::production;
use crate::{discord, line};

// AppRegistry is the main application state.
#[derive(Clone)]
pub struct AppRegistry {
    repository: Arc<dyn AppRepository>,
    notifier: Arc<dyn Notifier>,
    line_reply_sender: Arc<dyn LineReplySender>,
    staff_api_token: Arc<str>,
    line_channel_secret: Arc<str>,
    mutation_lock: Arc<Mutex<()>>,
    dispatch_lock: Arc<Mutex<()>>,
}

pub struct LineDiscordNotifier {
    line: Arc<Mutex<LINE>>,
    discord_ctx: Arc<Mutex<Context>>,
}

impl AppRegistry {
    pub fn new(
        line_token: String,
        line_channel_secret: String,
        staff_api_token: String,
        ctx: Context,
        repository: SqliteRepository,
    ) -> Self {
        let notifier = LineDiscordNotifier {
            line: Arc::new(Mutex::new(LINE::new(line_token))),
            discord_ctx: Arc::new(Mutex::new(ctx)),
        };
        let notifier = Arc::new(notifier);
        Self::new_with_ports(
            Arc::new(repository),
            notifier.clone(),
            notifier,
            staff_api_token,
            line_channel_secret,
        )
    }

    pub fn new_with_ports(
        repository: Arc<dyn AppRepository>,
        notifier: Arc<dyn Notifier>,
        line_reply_sender: Arc<dyn LineReplySender>,
        staff_api_token: String,
        line_channel_secret: String,
    ) -> Self {
        Self {
            repository,
            notifier,
            line_reply_sender,
            staff_api_token: Arc::from(staff_api_token),
            line_channel_secret: Arc::from(line_channel_secret),
            mutation_lock: Arc::new(Mutex::new(())),
            dispatch_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn staff_api_token(&self) -> &str {
        &self.staff_api_token
    }

    pub fn line_channel_secret(&self) -> &str {
        &self.line_channel_secret
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        self.repository.load_snapshot().await?;
        Ok(())
    }

    pub async fn snapshot(&self) -> anyhow::Result<OrderSystemSnapshot> {
        self.repository.load_snapshot().await
    }

    async fn mutate_snapshot<T>(
        &self,
        mutate: impl FnOnce(&mut OrderSystemSnapshot) -> T,
    ) -> anyhow::Result<T> {
        let _guard = self.mutation_lock.lock().await;
        let mut snapshot = self.repository.load_snapshot().await?;
        let result = mutate(&mut snapshot);
        self.repository.replace_snapshot(&snapshot).await?;
        Ok(result)
    }

    async fn send_notifications(&self, notifications: Vec<order_status::PendingNotification>) {
        self.dispatch_notifications(notifications).await;
    }

    pub async fn create_order(&self, items: Vec<Item>, is_priority: bool) -> anyhow::Result<Order> {
        let mutation = self
            .mutate_snapshot(|snapshot| order::create_order(snapshot, items, is_priority))
            .await?;
        self.send_notifications(mutation.status_update.notifications)
            .await;
        Ok(mutation.result)
    }

    // Updates stock and fulfills waiting orders.
    pub async fn update_production(
        &self,
        production: Vec<Item>,
    ) -> anyhow::Result<(Vec<u32>, Vec<Item>)> {
        let (status_update, unallocated_items) = self
            .mutate_snapshot(|snapshot| {
                let status_update = production::register_completed_production(snapshot, production);
                let unallocated_items = snapshot
                    .unallocated_stock
                    .iter()
                    .filter(|&(_, &quantity)| quantity > 0)
                    .map(|(flavor, &quantity)| Item { flavor, quantity })
                    .collect();
                (status_update, unallocated_items)
            })
            .await?;
        self.send_notifications(status_update.notifications).await;
        Ok((status_update.newly_ready_orders, unallocated_items))
    }

    pub async fn complete_order(&self, id: u32) -> anyhow::Result<Option<Order>> {
        let Some(mutation) = self
            .mutate_snapshot(|snapshot| order::complete_order(snapshot, id))
            .await?
        else {
            return Ok(None);
        };
        self.send_notifications(mutation.status_update.notifications)
            .await;
        Ok(Some(mutation.result))
    }

    pub async fn cancel_order(&self, id: u32) -> anyhow::Result<Option<Order>> {
        let Some(mutation) = self
            .mutate_snapshot(|snapshot| order::cancel_order(snapshot, id))
            .await?
        else {
            return Ok(None);
        };
        self.send_notifications(mutation.status_update.notifications)
            .await;
        Ok(Some(mutation.result))
    }

    pub async fn update_order_priority(
        &self,
        id: u32,
        is_priority: bool,
    ) -> anyhow::Result<Option<Order>> {
        let Some(mutation) = self
            .mutate_snapshot(|snapshot| order::update_order_priority(snapshot, id, is_priority))
            .await?
        else {
            return Ok(None);
        };
        self.send_notifications(mutation.status_update.notifications)
            .await;
        Ok(Some(mutation.result))
    }

    pub async fn add_notification(
        &self,
        id: u32,
        payload: Notify,
    ) -> anyhow::Result<Option<Order>> {
        self.mutate_snapshot(|snapshot| order::add_notification(snapshot, id, payload))
            .await
    }

    pub async fn cancel_notification(
        &self,
        id: u32,
        payload: &Notify,
    ) -> anyhow::Result<Option<Order>> {
        self.mutate_snapshot(|snapshot| order::cancel_notification(snapshot, id, payload))
            .await
    }

    pub async fn send_notification(&self, order_id: u32, notify: &Notify, message: String) {
        let notification = order_status::PendingNotification {
            order_id,
            notify: notify.clone(),
            message,
        };
        self.send_notifications(vec![notification]).await;
    }

    pub async fn reply_line_message(
        &self,
        reply_token: String,
        messages: Vec<Message>,
    ) -> anyhow::Result<()> {
        self.line_reply_sender.reply(reply_token, messages).await
    }

    async fn dispatch_notifications(&self, notifications: Vec<order_status::PendingNotification>) {
        let _guard = self.dispatch_lock.lock().await;

        for notification in notifications {
            let delivery = match self
                .notifier
                .send(notification.notify.clone(), notification.message.clone())
                .await
            {
                Ok(()) => NotificationDeliveryLog {
                    order_id: notification.order_id,
                    target: notification.notify,
                    message: notification.message,
                    status: NotificationDeliveryStatus::Sent,
                    error_message: None,
                },
                Err(error) => {
                    error!(
                        ?error,
                        order_id = notification.order_id,
                        "failed to send notification"
                    );
                    NotificationDeliveryLog {
                        order_id: notification.order_id,
                        target: notification.notify,
                        message: notification.message,
                        status: NotificationDeliveryStatus::Failed,
                        error_message: Some(error.to_string()),
                    }
                }
            };

            if let Err(error) = self
                .repository
                .record_notification_delivery(&delivery)
                .await
            {
                error!(
                    ?error,
                    order_id = delivery.order_id,
                    "failed to record notification delivery"
                );
            }
        }
    }
}

#[async_trait]
impl Notifier for LineDiscordNotifier {
    async fn send(&self, target: Notify, message: String) -> anyhow::Result<()> {
        match target {
            Notify::Discord {
                channel_id,
                user_id,
            } => {
                let ctx = self.discord_ctx.lock().await;
                if user_id != 0 {
                    discord::send_notification(&ctx, channel_id, user_id, &message).await?;
                }
            }
            Notify::Line { user_id } => {
                let line = self.line.lock().await;
                line::send_notification(line, user_id.clone(), message).await;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl LineReplySender for LineDiscordNotifier {
    async fn reply(&self, reply_token: String, messages: Vec<Message>) -> anyhow::Result<()> {
        let request = ReplyMessageRequest {
            reply_token,
            messages,
            notification_disabled: Some(false),
        };

        self.line
            .lock()
            .await
            .messaging_api_client
            .reply_message(request)
            .await
            .map_err(|error| anyhow::anyhow!("failed to reply line message: {error:?}"))?;
        Ok(())
    }
}

impl AppRegistry {
    pub async fn get_order_details(&self, id: u32) -> anyhow::Result<Option<OrderDetailsResponse>> {
        let snapshot = self.repository.load_snapshot().await?;
        let Some(order) = snapshot.orders.iter().find(|o| o.id == id) else {
            return Ok(None);
        };
        let estimated_wait_minutes = wait_time::estimate_order_wait_minutes(&snapshot, order);

        Ok(Some(OrderDetailsResponse {
            id: order.id,
            display_number: DisplayOrderNumber::from_order_id(order.id).as_str(),
            items: order.items.clone(),
            status: order.status,
            ordered_at: order.ordered_at,
            estimated_wait_minutes,
        }))
    }

    pub async fn get_current_wait_times(&self) -> anyhow::Result<WaitTimeResponse> {
        let snapshot = self.repository.load_snapshot().await?;
        Ok(WaitTimeResponse {
            wait_times: wait_time::estimate_current_wait_times(&snapshot),
        })
    }

    pub async fn set_flavor_config(
        &self,
        flavor: Flavor,
        config: FlavorConfig,
    ) -> anyhow::Result<()> {
        let status_update = self
            .mutate_snapshot(|snapshot| order::set_flavor_config(snapshot, flavor, config))
            .await?;
        self.send_notifications(status_update.notifications).await;
        Ok(())
    }
}
