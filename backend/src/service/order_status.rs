use std::collections::{HashMap, HashSet};

use chrono::Utc;
use enum_map::EnumMap;

use crate::data::{Data, Flavor, Notify, Order, OrderStatus};

pub struct PendingNotification {
    pub order_id: u32,
    pub notify: Notify,
    pub message: String,
}

pub struct StatusUpdate {
    pub newly_ready_orders: Vec<u32>,
    pub notifications: Vec<PendingNotification>,
}

pub fn update_order_statuses(data: &mut Data) -> StatusUpdate {
    let mut newly_ready_orders = Vec::new();
    let mut notifications = Vec::new();

    let previously_cooking_order_ids = data
        .orders
        .iter()
        .filter(|o| o.status == OrderStatus::Cooking)
        .map(|o| o.id)
        .collect::<HashSet<_>>();

    for order in data
        .orders
        .iter_mut()
        .filter(|o| o.status == OrderStatus::Cooking)
    {
        order.status = OrderStatus::Waiting;
    }

    let mut stock = std::mem::take(&mut data.unallocated_stock);
    let mut waiting_order_indices = data
        .orders
        .iter()
        .enumerate()
        .filter(|(_, o)| o.status == OrderStatus::Waiting)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    waiting_order_indices
        .sort_by_key(|&i| (!data.orders[i].is_priority, data.orders[i].ordered_at));

    for index in waiting_order_indices {
        let order = &mut data.orders[index];
        if can_fulfill(order, &stock) {
            fulfill(order, &mut stock);
            order.status = OrderStatus::Ready;
            order.ready_at.replace(Utc::now());
            newly_ready_orders.push(order.id);
            notifications.extend(
                order
                    .notify
                    .iter()
                    .cloned()
                    .map(|notify| PendingNotification {
                        order_id: order.id,
                        notify,
                        message: format!("#{}番 のご注文の準備ができました！", order.id),
                    }),
            );
        }
    }
    data.unallocated_stock = stock;

    let mut waiting_orders = data
        .orders
        .iter_mut()
        .filter(|o| o.status == OrderStatus::Waiting)
        .collect::<Vec<_>>();
    waiting_orders.sort_by_key(|o| (!o.is_priority, o.ordered_at));

    let mut cumulative_demand = HashMap::new();

    for order in waiting_orders {
        let mut is_cooking = !order.items.is_empty();

        for item in &order.items {
            let demand_so_far = cumulative_demand.get(&item.flavor).copied().unwrap_or(0);
            let current_stock = data.unallocated_stock[item.flavor];
            let total_demand_for_item = demand_so_far + item.quantity;
            let needed_from_production = total_demand_for_item.saturating_sub(current_stock);

            if needed_from_production > 0 {
                let config = data.flavor_configs[item.flavor];
                if needed_from_production > config.quantity_per_batch as usize {
                    is_cooking = false;
                    break;
                }
            }
        }

        if is_cooking {
            order.status = OrderStatus::Cooking;
            if !previously_cooking_order_ids.contains(&order.id) {
                notifications.extend(order.notify.iter().cloned().map(|notify| {
                    PendingNotification {
                        order_id: order.id,
                        notify,
                        message: format!(
                            "#{}番 調理中です！\n遠くにいる場合は近くでお待ちください。",
                            order.id
                        ),
                    }
                }));
            }
        }

        for item in &order.items {
            *cumulative_demand.entry(item.flavor).or_insert(0) += item.quantity;
        }
    }

    StatusUpdate {
        newly_ready_orders,
        notifications,
    }
}

fn can_fulfill(order: &Order, stock: &EnumMap<Flavor, usize>) -> bool {
    order
        .items
        .iter()
        .all(|item| stock[item.flavor] >= item.quantity)
}

fn fulfill(order: &Order, stock: &mut EnumMap<Flavor, usize>) {
    for item in &order.items {
        stock[item.flavor] -= item.quantity;
    }
}
