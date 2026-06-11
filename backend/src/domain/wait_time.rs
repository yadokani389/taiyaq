use enum_map::EnumMap;
use strum::IntoEnumIterator;

use crate::domain::snapshot::{Flavor, Order, OrderStatus, OrderSystemSnapshot};

pub fn estimate_order_wait_minutes(snapshot: &OrderSystemSnapshot, order: &Order) -> Option<i64> {
    if order.status != OrderStatus::Waiting {
        return None;
    }

    order
        .items
        .iter()
        .map(|item_in_order| {
            let flavor = item_in_order.flavor;
            let total_demand = snapshot
                .orders
                .iter()
                .filter(|other| {
                    other.status == OrderStatus::Waiting
                        && order_priority_key(other) <= order_priority_key(order)
                })
                .flat_map(|other| &other.items)
                .filter(|item| item.flavor == flavor)
                .map(|item| item.quantity)
                .sum::<usize>();

            estimate_flavor_wait_minutes(snapshot, flavor, total_demand)
        })
        .max()
        .unwrap_or(Some(0))
}

pub fn estimate_current_wait_times(snapshot: &OrderSystemSnapshot) -> EnumMap<Flavor, Option<i64>> {
    let mut wait_times = EnumMap::from_fn(|_| None);

    for flavor in Flavor::iter() {
        let demand = snapshot
            .orders
            .iter()
            .filter(|order| {
                order.status == OrderStatus::Waiting || order.status == OrderStatus::Cooking
            })
            .flat_map(|order| &order.items)
            .filter(|item| item.flavor == flavor)
            .map(|item| item.quantity)
            .sum::<usize>()
            + 1;

        wait_times[flavor] = estimate_flavor_wait_minutes(snapshot, flavor, demand);
    }

    wait_times
}

fn order_priority_key(order: &Order) -> (bool, chrono::DateTime<chrono::Utc>, u32) {
    (!order.is_priority, order.ordered_at, order.id)
}

fn estimate_flavor_wait_minutes(
    snapshot: &OrderSystemSnapshot,
    flavor: Flavor,
    demand: usize,
) -> Option<i64> {
    let stock = snapshot.unallocated_stock[flavor];
    if demand <= stock {
        return Some(0);
    }

    let needed_from_production = demand.saturating_sub(stock);
    let config = snapshot.flavor_configs[flavor];
    if config.quantity_per_batch == 0 {
        return None;
    }

    let batches_needed = needed_from_production.div_ceil(config.quantity_per_batch as usize);
    Some(batches_needed as i64 * config.cooking_time_minutes as i64)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::domain::snapshot::{Flavor, Item, Order, OrderStatus, OrderSystemSnapshot};

    use super::{estimate_current_wait_times, estimate_order_wait_minutes};

    fn waiting_order(id: u32, quantity: usize) -> Order {
        Order {
            id,
            items: vec![Item {
                flavor: Flavor::Tsubuan,
                quantity,
            }],
            status: OrderStatus::Waiting,
            ordered_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, id).unwrap(),
            ready_at: None,
            completed_at: None,
            notify: Default::default(),
            is_priority: false,
        }
    }

    fn priority_waiting_order(id: u32, quantity: usize) -> Order {
        Order {
            is_priority: true,
            ..waiting_order(id, quantity)
        }
    }

    #[test]
    fn current_wait_time_is_zero_when_stock_covers_next_order() {
        let mut snapshot = OrderSystemSnapshot::default();
        snapshot.unallocated_stock[Flavor::Tsubuan] = 1;

        let wait_times = estimate_current_wait_times(&snapshot);

        assert_eq!(wait_times[Flavor::Tsubuan], Some(0));
    }

    #[test]
    fn waiting_order_uses_largest_flavor_wait_time() {
        let snapshot = OrderSystemSnapshot {
            orders: vec![waiting_order(1, 10)],
            ..Default::default()
        };

        assert_eq!(
            estimate_order_wait_minutes(&snapshot, &snapshot.orders[0]),
            Some(30)
        );
    }

    #[test]
    fn priority_order_wait_time_ignores_overtaken_normal_orders() {
        let snapshot = OrderSystemSnapshot {
            orders: vec![waiting_order(1, 9), priority_waiting_order(2, 1)],
            ..Default::default()
        };

        assert_eq!(
            estimate_order_wait_minutes(&snapshot, &snapshot.orders[1]),
            Some(15)
        );
    }
}
