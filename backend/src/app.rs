use std::sync::Arc;

use bot_sdk_line::client::LINE;
use chrono::Utc;
use enum_map::EnumMap;
use poise::serenity_prelude::Context;
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

use crate::api::model::{OrderDetailsResponse, WaitTimeResponse};
use crate::data::{Data, Flavor, FlavorConfig, Item, Notify, Order, OrderStatus};
use crate::service::order_status;
use crate::storage::SqliteRepository;
use crate::{discord, line};

// AppRegistry is the main application state.
#[derive(Clone)]
pub struct AppRegistry {
    data: Arc<RwLock<Data>>,
    repository: Arc<SqliteRepository>,
    pub line: Arc<Mutex<LINE>>,
    pub discord_ctx: Arc<Mutex<Context>>,
}

impl AppRegistry {
    pub fn new(line_token: String, ctx: Context, repository: SqliteRepository) -> Self {
        Self {
            data: Arc::new(RwLock::new(Data::default())),
            repository: Arc::new(repository),
            line: Arc::new(Mutex::new(LINE::new(line_token))),
            discord_ctx: Arc::new(Mutex::new(ctx)),
        }
    }

    pub async fn save_data(&self) -> anyhow::Result<()> {
        self.repository.save(&*self.data.read().await).await?;
        Ok(())
    }

    pub async fn load_data(&self) -> anyhow::Result<()> {
        let data = self.repository.load().await?;
        *self.data.write().await = data;
        Ok(())
    }

    pub async fn data(&self) -> RwLockReadGuard<'_, Data> {
        self.data.read().await
    }

    async fn send_notifications(&self, notifications: Vec<order_status::PendingNotification>) {
        for notification in notifications {
            self.send_notification(
                notification.order_id,
                &notification.notify,
                notification.message,
            )
            .await;
        }
    }

    pub async fn create_order(&self, items: Vec<Item>, is_priority: bool) -> anyhow::Result<Order> {
        let mut data = self.data.write().await;
        let new_id = data.orders.iter().map(|o| o.id).max().unwrap_or(0) + 1;
        let new_order = Order {
            id: new_id,
            items,
            status: OrderStatus::Waiting, // Start as waiting
            ordered_at: Utc::now(),
            ready_at: None,
            completed_at: None,
            notify: Default::default(),
            is_priority,
        };
        data.orders.push(new_order);

        // Recalculate statuses
        let status_update = order_status::update_order_statuses(&mut data);

        // Find the order we just added to return its (potentially updated) state
        let result_order = data.orders.iter().find(|o| o.id == new_id).unwrap().clone();

        drop(data);
        self.save_data().await?;
        self.send_notifications(status_update.notifications).await;
        Ok(result_order)
    }

    // Updates stock and fulfills waiting orders.
    pub async fn update_production(
        &self,
        production: Vec<Item>,
    ) -> anyhow::Result<(Vec<u32>, Vec<Item>)> {
        let mut data = self.data.write().await;

        // Add new production to stock
        for item in production {
            data.unallocated_stock[item.flavor] += item.quantity;
        }

        // Recalculate statuses
        let status_update = order_status::update_order_statuses(&mut data);

        // Collect unallocated items for the response
        let unallocated_items = data
            .unallocated_stock
            .iter()
            .filter(|&(_, &quantity)| quantity > 0)
            .map(|(flavor, &quantity)| Item { flavor, quantity })
            .collect();

        drop(data);
        self.save_data().await?;
        self.send_notifications(status_update.notifications).await;
        Ok((status_update.newly_ready_orders, unallocated_items))
    }

    pub async fn complete_order(&self, id: u32) -> anyhow::Result<Option<Order>> {
        let mut data = self.data.write().await;
        let Some(order) = data.orders.iter_mut().find(|o| o.id == id) else {
            return Ok(None);
        };
        let previous_status = order.status;
        order.status = OrderStatus::Completed;
        order.completed_at = Some(Utc::now());

        let order = order.clone();

        if matches!(
            previous_status,
            OrderStatus::Waiting | OrderStatus::Cooking | OrderStatus::Ready
        ) {
            let status_update = order_status::update_order_statuses(&mut data);
            drop(data);
            self.save_data().await?;
            self.send_notifications(status_update.notifications).await;
            return Ok(Some(order));
        }

        drop(data);
        self.save_data().await?;
        Ok(Some(order))
    }

    pub async fn cancel_order(&self, id: u32) -> anyhow::Result<Option<Order>> {
        let mut data = self.data.write().await;

        // Find the index of the order to avoid borrowing issues
        let Some(order_idx) = data.orders.iter().position(|o| o.id == id) else {
            return Ok(None);
        };
        let previous_status = data.orders[order_idx].status;
        let items_to_return = if previous_status == OrderStatus::Ready {
            // Clone the items to release the borrow on `data.orders`
            Some(data.orders[order_idx].items.clone())
        } else {
            None
        };

        // Now, modify the order status
        data.orders[order_idx].status = OrderStatus::Cancelled;
        let cancelled_order = data.orders[order_idx].clone();

        let stock_was_changed = if let Some(items) = items_to_return {
            // Now we can mutably borrow `data.unallocated_stock` because the borrow for `items` is gone
            for item in items {
                data.unallocated_stock[item.flavor] += item.quantity;
            }
            true
        } else {
            false
        };

        // If demand or stock changed, other orders might have become ready or cooking.
        if stock_was_changed
            || matches!(previous_status, OrderStatus::Waiting | OrderStatus::Cooking)
        {
            let status_update = order_status::update_order_statuses(&mut data);
            drop(data);
            self.save_data().await?;
            self.send_notifications(status_update.notifications).await;
            return Ok(Some(cancelled_order));
        }

        drop(data);
        self.save_data().await?;
        Ok(Some(cancelled_order))
    }

    pub async fn update_order_priority(
        &self,
        id: u32,
        is_priority: bool,
    ) -> anyhow::Result<Option<Order>> {
        let mut data = self.data.write().await;
        let Some(order) = data.orders.iter_mut().find(|o| o.id == id) else {
            return Ok(None);
        };

        // Do nothing if the priority is already set to the desired value.
        if order.is_priority == is_priority {
            return Ok(Some(order.clone()));
        }

        order.is_priority = is_priority;

        // Recalculate statuses as priority change can affect order processing sequence.
        let status_update = order_status::update_order_statuses(&mut data);

        let updated_order = data.orders.iter().find(|o| o.id == id).unwrap().clone();

        drop(data);
        self.save_data().await?;
        self.send_notifications(status_update.notifications).await;
        Ok(Some(updated_order))
    }

    pub async fn add_notification(
        &self,
        id: u32,
        payload: Notify,
    ) -> anyhow::Result<Option<Order>> {
        let mut data = self.data.write().await;
        let Some(order) = data.orders.iter_mut().find(|o| o.id == id) else {
            return Ok(None);
        };
        order.notify.insert(payload);
        let order = order.clone();
        drop(data);
        self.save_data().await?;
        Ok(Some(order))
    }

    pub async fn cancel_notification(
        &self,
        id: u32,
        payload: &Notify,
    ) -> anyhow::Result<Option<Order>> {
        let mut data = self.data.write().await;
        let Some(order) = data.orders.iter_mut().find(|o| o.id == id) else {
            return Ok(None);
        };
        order.notify.remove(payload);
        let order = order.clone();
        drop(data);
        self.save_data().await?;
        Ok(Some(order))
    }

    pub async fn send_notification(&self, order_id: u32, notify: &Notify, message: String) {
        // TODO: This is a placeholder.
        println!(
            "Sending notification for Order ID: {}, Target: {:?}, Message: {}",
            order_id, notify, message
        );

        match notify {
            Notify::Discord {
                channel_id,
                user_id,
            } => {
                let ctx = self.discord_ctx.lock().await;
                if *user_id != 0 {
                    discord::send_notification(&ctx, *channel_id, *user_id, &message)
                        .await
                        .ok();
                }
            }
            Notify::Line { user_id } => {
                let line = self.line.lock().await;
                line::send_notification(line, user_id.clone(), message).await;
            }
        }
    }

    pub async fn get_order_details(&self, id: u32) -> Option<OrderDetailsResponse> {
        let data = self.data.read().await;
        let order = data.orders.iter().find(|o| o.id == id)?;

        let estimated_wait_minutes = if order.status == OrderStatus::Waiting {
            let mut max_wait_time: i64 = 0;

            // For each item type in the order we are querying for...
            for item_in_order in &order.items {
                let flavor_to_calc = item_in_order.flavor;

                // 1. Calculate total demand for this flavor from all waiting orders placed
                //    at or before the current order.
                let total_demand_for_flavor = data
                    .orders
                    .iter()
                    .filter(|o| {
                        o.status == OrderStatus::Waiting && o.ordered_at <= order.ordered_at
                    })
                    .flat_map(|o| &o.items)
                    .filter(|item| item.flavor == flavor_to_calc)
                    .map(|item| item.quantity)
                    .sum::<usize>();

                // 2. Subtract available stock.
                let stock_for_flavor = data.unallocated_stock[flavor_to_calc];
                let needed_from_production =
                    total_demand_for_flavor.saturating_sub(stock_for_flavor);

                if needed_from_production == 0 {
                    continue; // This flavor doesn't contribute to the wait time.
                }

                // 3. Calculate batches and time based on config.
                let wait_time_for_flavor = {
                    let config = data.flavor_configs[flavor_to_calc];
                    if config.quantity_per_batch > 0 {
                        let batches_needed =
                            needed_from_production.div_ceil(config.quantity_per_batch as usize);
                        batches_needed as i64 * config.cooking_time_minutes as i64
                    } else {
                        0 // Avoid division by zero, assume no wait time if batch size is 0.
                    }
                };

                // 4. Update the max wait time for the order.
                if wait_time_for_flavor > max_wait_time {
                    max_wait_time = wait_time_for_flavor;
                }
            }

            Some(max_wait_time)
        } else {
            None
        };

        Some(OrderDetailsResponse {
            id: order.id,
            items: order.items.clone(),
            status: order.status,
            ordered_at: order.ordered_at,
            estimated_wait_minutes,
        })
    }

    pub async fn get_current_wait_times(&self) -> WaitTimeResponse {
        let data = self.data.read().await;
        let mut wait_times = EnumMap::from_fn(|_| None);

        for flavor in Flavor::iter() {
            // 1. Calculate total demand for this flavor from all waiting/cooking orders.
            let demand_before_me = data
                .orders
                .iter()
                .filter(|o| o.status == OrderStatus::Waiting || o.status == OrderStatus::Cooking)
                .flat_map(|o| &o.items)
                .filter(|item| item.flavor == flavor)
                .map(|item| item.quantity)
                .sum::<usize>();

            // We are calculating for a hypothetical new order of 1 item.
            let total_demand = demand_before_me + 1;

            // 2. Subtract available stock.
            let stock_for_flavor = data.unallocated_stock[flavor];

            let estimated_wait_minutes = if total_demand <= stock_for_flavor {
                Some(0)
            } else {
                let needed_from_production = total_demand.saturating_sub(stock_for_flavor);

                // 3. Calculate batches and time based on config.
                let config = data.flavor_configs[flavor];
                if config.quantity_per_batch > 0 {
                    let batches_needed =
                        needed_from_production.div_ceil(config.quantity_per_batch as usize);
                    Some(batches_needed as i64 * config.cooking_time_minutes as i64)
                } else {
                    None // Cannot be produced
                }
            };

            wait_times[flavor] = estimated_wait_minutes;
        }

        WaitTimeResponse { wait_times }
    }

    pub async fn set_flavor_config(
        &self,
        flavor: Flavor,
        config: FlavorConfig,
    ) -> anyhow::Result<()> {
        let mut data = self.data.write().await;
        data.flavor_configs[flavor] = config;
        let status_update = order_status::update_order_statuses(&mut data);
        drop(data);
        self.save_data().await?;
        self.send_notifications(status_update.notifications).await;
        Ok(())
    }
}
