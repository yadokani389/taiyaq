use std::collections::HashMap;
use std::sync::Arc;

use bot_sdk_line::client::LINE;
use chrono::Utc;
use enum_map::EnumMap;
use poise::serenity_prelude::Context;
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

use crate::api::model::{OrderDetailsResponse, WaitTimeResponse};
use crate::data::{Data, Flavor, FlavorConfig, Item, Notify, Order, OrderStatus};
use crate::{discord, line};

// AppRegistry is the main application state.
#[derive(Clone)]
pub struct AppRegistry {
    data: Arc<RwLock<Data>>,
    pub line: Arc<Mutex<LINE>>,
    pub discord_ctx: Arc<Mutex<Context>>,
}

impl AppRegistry {
    const FILE_PATH: &str = "data.json";

    pub fn new(line_token: String, ctx: Context) -> Self {
        Self {
            data: Arc::new(RwLock::new(Data::default())),
            line: Arc::new(Mutex::new(LINE::new(line_token))),
            discord_ctx: Arc::new(Mutex::new(ctx)),
        }
    }

    pub async fn save_data(&self) -> anyhow::Result<()> {
        let data_str = serde_json::to_string_pretty(&*self.data.read().await)?;
        std::fs::write(Self::FILE_PATH, data_str)?;
        Ok(())
    }

    pub async fn load_data(&self) -> anyhow::Result<()> {
        let data_str = std::fs::read_to_string(Self::FILE_PATH)?;
        let data: Data = serde_json::from_str(&data_str)?;
        *self.data.write().await = data;
        Ok(())
    }

    pub async fn data(&self) -> RwLockReadGuard<'_, Data> {
        self.data.read().await
    }

    // Recalculates and updates the status for all orders based on current stock and demand.
    // This is the core logic for transitioning orders to 'Ready' or 'Cooking'.
    // Returns a list of order IDs that have newly become 'Ready'.
    async fn update_order_statuses(&self, data: &mut Data) -> Vec<u32> {
        let mut newly_ready_orders = Vec::new();

        // Part 1: Reset remaining Cooking orders to Waiting to prepare for recalculation
        for order in data
            .orders
            .iter_mut()
            .filter(|o| o.status == OrderStatus::Cooking)
        {
            order.status = OrderStatus::Waiting;
        }

        // Part 2: Fulfill what can be fulfilled now (Waiting -> Ready)
        let mut stock = std::mem::take(&mut data.unallocated_stock);

        // Create a list of order indices to iterate over, to avoid borrowing issues.
        let mut waiting_order_indices: Vec<usize> = data
            .orders
            .iter()
            .enumerate()
            .filter(|(_, o)| o.status == OrderStatus::Waiting)
            .map(|(i, _)| i)
            .collect();

        // Sort indices by priority (true first, so use reverse) and then by ordered_at.
        waiting_order_indices
            .sort_by_key(|&i| (!data.orders[i].is_priority, data.orders[i].ordered_at));

        for index in waiting_order_indices {
            let order = &mut data.orders[index];
            if Self::can_fulfill(order, &stock) {
                Self::fulfill(order, &mut stock);
                order.status = OrderStatus::Ready;
                order.ready_at.replace(Utc::now());
                newly_ready_orders.push(order.id);
                for notify in &order.notify {
                    self.send_notification(
                        order.id,
                        notify,
                        format!("#{}番 のご注文の準備ができました！", order.id),
                    )
                    .await;
                }
            }
        }
        data.unallocated_stock = stock; // Put the remaining stock back

        // Part 3: Find new 'Cooking' orders from the now-complete 'Waiting' pool
        let mut waiting_orders: Vec<&mut Order> = data
            .orders
            .iter_mut()
            .filter(|o| o.status == OrderStatus::Waiting)
            .collect();
        // Sort by priority (true first, so use reverse), then by time.
        waiting_orders.sort_by_key(|o| (!o.is_priority, o.ordered_at));

        let mut cumulative_demand: HashMap<Flavor, usize> = HashMap::new();

        for order in waiting_orders {
            let mut is_cooking = !order.items.is_empty();

            for item in &order.items {
                let demand_so_far = cumulative_demand.get(&item.flavor).copied().unwrap_or(0);
                let current_stock = data.unallocated_stock[item.flavor];

                // How many items need to be produced to fulfill demand up to this point (including current item)
                let total_demand_for_item = demand_so_far + item.quantity;
                let needed_from_production = total_demand_for_item.saturating_sub(current_stock);

                if needed_from_production > 0 {
                    let config = data.flavor_configs[item.flavor];
                    // If the items needed from production exceed what the first batch can provide,
                    // then this order is not in the "cooking" phase.
                    if needed_from_production > config.quantity_per_batch as usize {
                        is_cooking = false;
                        break; // No need to check other items in this order
                    }
                } else {
                    // No config for this flavor, so it can't be determined to be 'cooking'.
                    is_cooking = false;
                    break;
                }
            }

            if is_cooking {
                order.status = OrderStatus::Cooking;
            }

            // Update cumulative demand for the next iteration
            for item in &order.items {
                *cumulative_demand.entry(item.flavor).or_insert(0) += item.quantity;
            }
        }

        newly_ready_orders
    }

    pub async fn create_order(&self, items: Vec<Item>, is_priority: bool) -> Order {
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
        self.update_order_statuses(&mut data).await;

        // Find the order we just added to return its (potentially updated) state
        let result_order = data.orders.iter().find(|o| o.id == new_id).unwrap().clone();

        drop(data);
        self.save_data().await.ok();
        result_order
    }

    // Updates stock and fulfills waiting orders.
    pub async fn update_production(&self, production: Vec<Item>) -> (Vec<u32>, Vec<Item>) {
        let mut data = self.data.write().await;

        // Add new production to stock
        for item in production {
            data.unallocated_stock[item.flavor] += item.quantity;
        }

        // Recalculate statuses
        let newly_ready_orders = self.update_order_statuses(&mut data).await;

        // Collect unallocated items for the response
        let unallocated_items = data
            .unallocated_stock
            .iter()
            .filter(|&(_, &quantity)| quantity > 0)
            .map(|(flavor, &quantity)| Item { flavor, quantity })
            .collect();

        drop(data);
        self.save_data().await.ok();
        (newly_ready_orders, unallocated_items)
    }

    // Helper to check if an order can be fulfilled from stock
    fn can_fulfill(order: &Order, stock: &EnumMap<Flavor, usize>) -> bool {
        order
            .items
            .iter()
            .all(|item| stock[item.flavor] >= item.quantity)
    }

    // Helper to decrement stock for a fulfilled order
    fn fulfill(order: &Order, stock: &mut EnumMap<Flavor, usize>) {
        for item in &order.items {
            stock[item.flavor] -= item.quantity;
        }
    }

    pub async fn complete_order(&self, id: u32) -> Option<Order> {
        let mut data = self.data.write().await;
        let order = data.orders.iter_mut().find(|o| o.id == id)?;
        order.status = OrderStatus::Completed;
        order.completed_at = Some(Utc::now());

        let order = order.clone();
        drop(data);
        self.save_data().await.ok();
        Some(order)
    }

    pub async fn cancel_order(&self, id: u32) -> Option<Order> {
        let mut data = self.data.write().await;

        // Find the index of the order to avoid borrowing issues
        let order_idx = data.orders.iter().position(|o| o.id == id)?;
        let items_to_return = if data.orders[order_idx].status == OrderStatus::Ready {
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

        // If stock was changed, other orders might have become ready or cooking
        if stock_was_changed {
            self.update_order_statuses(&mut data).await;
        }

        drop(data);
        self.save_data().await.ok();
        Some(cancelled_order)
    }

    pub async fn update_order_priority(&self, id: u32, is_priority: bool) -> Option<Order> {
        let mut data = self.data.write().await;
        let order = data.orders.iter_mut().find(|o| o.id == id)?;

        // Do nothing if the priority is already set to the desired value.
        if order.is_priority == is_priority {
            return Some(order.clone());
        }

        order.is_priority = is_priority;

        // Recalculate statuses as priority change can affect order processing sequence.
        self.update_order_statuses(&mut data).await;

        let updated_order = data.orders.iter().find(|o| o.id == id).unwrap().clone();

        drop(data);
        self.save_data().await.ok();
        Some(updated_order)
    }

    pub async fn add_notification(&self, id: u32, payload: Notify) -> Option<Order> {
        let mut data = self.data.write().await;
        let order = data.orders.iter_mut().find(|o| o.id == id)?;
        order.notify.insert(payload);
        let order = order.clone();
        drop(data);
        self.save_data().await.ok();
        Some(order)
    }

    pub async fn cancel_notification(&self, id: u32, payload: &Notify) -> Option<Order> {
        let mut data = self.data.write().await;
        let order = data.orders.iter_mut().find(|o| o.id == id)?;
        order.notify.remove(payload);
        let order = order.clone();
        drop(data);
        self.save_data().await.ok();
        Some(order)
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

    pub async fn set_flavor_config(&self, flavor: Flavor, config: FlavorConfig) {
        let mut data = self.data.write().await;
        data.flavor_configs[flavor] = config;
        drop(data);
        self.save_data().await.ok();
    }
}
