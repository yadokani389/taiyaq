use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use enum_map::EnumMap;

use crate::domain::order_number::DisplayOrderNumber;
use crate::domain::snapshot::{Flavor, Notify, Order, OrderStatus, OrderSystemSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingNotification {
    pub order_id: u32,
    pub notify: Notify,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusUpdate {
    pub newly_ready_orders: Vec<u32>,
    pub notifications: Vec<PendingNotification>,
}

pub fn update_order_statuses(snapshot: &mut OrderSystemSnapshot) -> StatusUpdate {
    update_order_statuses_at(snapshot, Utc::now())
}

fn update_order_statuses_at(
    snapshot: &mut OrderSystemSnapshot,
    now: DateTime<Utc>,
) -> StatusUpdate {
    let mut newly_ready_orders = Vec::new();
    let mut notifications = Vec::new();

    let previously_cooking_order_ids = snapshot
        .orders
        .iter()
        .filter(|order| order.status == OrderStatus::Cooking)
        .map(|order| order.id)
        .collect::<HashSet<_>>();

    for order in snapshot
        .orders
        .iter_mut()
        .filter(|order| order.status == OrderStatus::Cooking)
    {
        order.status = OrderStatus::Waiting;
    }

    let mut stock = std::mem::take(&mut snapshot.unallocated_stock);
    let mut waiting_order_indices = snapshot
        .orders
        .iter()
        .enumerate()
        .filter(|(_, order)| order.status == OrderStatus::Waiting)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    waiting_order_indices.sort_by_key(|&index| {
        (
            !snapshot.orders[index].is_priority,
            snapshot.orders[index].ordered_at,
            snapshot.orders[index].id,
        )
    });

    for index in waiting_order_indices {
        let order = &mut snapshot.orders[index];
        if can_fulfill(order, &stock) {
            fulfill(order, &mut stock);
            order.status = OrderStatus::Ready;
            order.ready_at.replace(now);
            newly_ready_orders.push(order.id);
            notifications.extend(
                order
                    .notify
                    .iter()
                    .cloned()
                    .map(|notify| PendingNotification {
                        order_id: order.id,
                        notify,
                        message: format!(
                            "#{}番 のご注文の準備ができました！",
                            DisplayOrderNumber::from_order_id(order.id).as_str()
                        ),
                    }),
            );
        }
    }
    snapshot.unallocated_stock = stock;

    let mut waiting_orders = snapshot
        .orders
        .iter_mut()
        .filter(|order| order.status == OrderStatus::Waiting)
        .collect::<Vec<_>>();
    waiting_orders.sort_by_key(|order| (!order.is_priority, order.ordered_at, order.id));

    let mut cumulative_demand = HashMap::new();

    for order in waiting_orders {
        let mut is_cooking = !order.items.is_empty();

        for item in &order.items {
            let demand_so_far = cumulative_demand.get(&item.flavor).copied().unwrap_or(0);
            let current_stock = snapshot.unallocated_stock[item.flavor];
            let total_demand_for_item = demand_so_far + item.quantity;
            let needed_from_production = total_demand_for_item.saturating_sub(current_stock);

            if needed_from_production > 0 {
                let config = snapshot.flavor_configs[item.flavor];
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
                            DisplayOrderNumber::from_order_id(order.id).as_str()
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use crate::domain::snapshot::{Flavor, Item, Notify, Order, OrderStatus, OrderSystemSnapshot};

    use super::update_order_statuses_at;

    fn order(id: u32, is_priority: bool) -> Order {
        Order {
            id,
            items: vec![Item {
                flavor: Flavor::Tsubuan,
                quantity: 1,
            }],
            status: OrderStatus::Waiting,
            ordered_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
                + Duration::seconds(id.into()),
            ready_at: None,
            completed_at: None,
            notify: Default::default(),
            is_priority,
        }
    }

    #[test]
    fn priority_order_can_overtake_normal_order() {
        let mut snapshot = OrderSystemSnapshot {
            orders: vec![order(1, false), order(2, true)],
            ..Default::default()
        };
        snapshot.unallocated_stock[Flavor::Tsubuan] = 1;

        let update = update_order_statuses_at(
            &mut snapshot,
            Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap(),
        );

        assert_eq!(update.newly_ready_orders, vec![2]);
        assert_eq!(snapshot.orders[0].status, OrderStatus::Cooking);
        assert_eq!(snapshot.orders[1].status, OrderStatus::Ready);
    }

    #[test]
    fn ready_notification_uses_display_order_number() {
        let mut target = order(123, false);
        target.notify.insert(Notify::Line {
            user_id: "user".to_owned(),
        });
        let mut snapshot = OrderSystemSnapshot {
            orders: vec![target],
            ..Default::default()
        };
        snapshot.unallocated_stock[Flavor::Tsubuan] = 1;

        let update = update_order_statuses_at(
            &mut snapshot,
            Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap(),
        );

        assert_eq!(update.notifications.len(), 1);
        assert!(update.notifications[0].message.contains("#23番"));
    }
}
