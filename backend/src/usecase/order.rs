use chrono::Utc;

use crate::domain::order_status::{self, StatusUpdate};
use crate::domain::snapshot::{
    Flavor, FlavorConfig, Item, Notify, Order, OrderStatus, OrderSystemSnapshot,
};

pub struct OrderMutation<T> {
    pub result: T,
    pub status_update: StatusUpdate,
}

pub fn create_order(
    snapshot: &mut OrderSystemSnapshot,
    items: Vec<Item>,
    is_priority: bool,
) -> OrderMutation<Order> {
    let new_id = snapshot
        .orders
        .iter()
        .map(|order| order.id)
        .max()
        .unwrap_or(0)
        + 1;
    let new_order = Order {
        id: new_id,
        items,
        status: OrderStatus::Waiting,
        ordered_at: Utc::now(),
        ready_at: None,
        completed_at: None,
        notify: Default::default(),
        is_priority,
    };
    snapshot.orders.push(new_order);

    let status_update = order_status::update_order_statuses(snapshot);
    let result = snapshot
        .orders
        .iter()
        .find(|order| order.id == new_id)
        .expect("created order must exist")
        .clone();

    OrderMutation {
        result,
        status_update,
    }
}

pub fn complete_order(snapshot: &mut OrderSystemSnapshot, id: u32) -> Option<OrderMutation<Order>> {
    let order = snapshot.orders.iter_mut().find(|order| order.id == id)?;
    let previous_status = order.status;
    order.status = OrderStatus::Completed;
    order.completed_at = Some(Utc::now());
    let result = order.clone();

    let status_update = if matches!(
        previous_status,
        OrderStatus::Waiting | OrderStatus::Cooking | OrderStatus::Ready
    ) {
        order_status::update_order_statuses(snapshot)
    } else {
        empty_status_update()
    };

    Some(OrderMutation {
        result,
        status_update,
    })
}

pub fn cancel_order(snapshot: &mut OrderSystemSnapshot, id: u32) -> Option<OrderMutation<Order>> {
    let order_index = snapshot.orders.iter().position(|order| order.id == id)?;
    let previous_status = snapshot.orders[order_index].status;
    let items_to_return =
        (previous_status == OrderStatus::Ready).then(|| snapshot.orders[order_index].items.clone());

    snapshot.orders[order_index].status = OrderStatus::Cancelled;
    let result = snapshot.orders[order_index].clone();

    let stock_was_changed = if let Some(items) = items_to_return {
        for item in items {
            snapshot.unallocated_stock[item.flavor] += item.quantity;
        }
        true
    } else {
        false
    };

    let status_update = if stock_was_changed
        || matches!(previous_status, OrderStatus::Waiting | OrderStatus::Cooking)
    {
        order_status::update_order_statuses(snapshot)
    } else {
        empty_status_update()
    };

    Some(OrderMutation {
        result,
        status_update,
    })
}

pub fn update_order_priority(
    snapshot: &mut OrderSystemSnapshot,
    id: u32,
    is_priority: bool,
) -> Option<OrderMutation<Order>> {
    let order = snapshot.orders.iter_mut().find(|order| order.id == id)?;
    if order.is_priority == is_priority {
        return Some(OrderMutation {
            result: order.clone(),
            status_update: empty_status_update(),
        });
    }

    order.is_priority = is_priority;
    let status_update = order_status::update_order_statuses(snapshot);
    let result = snapshot
        .orders
        .iter()
        .find(|order| order.id == id)
        .expect("updated order must exist")
        .clone();

    Some(OrderMutation {
        result,
        status_update,
    })
}

pub fn add_notification(
    snapshot: &mut OrderSystemSnapshot,
    id: u32,
    notify: Notify,
) -> Option<Order> {
    let order = snapshot.orders.iter_mut().find(|order| order.id == id)?;
    order.notify.insert(notify);
    Some(order.clone())
}

pub fn cancel_notification(
    snapshot: &mut OrderSystemSnapshot,
    id: u32,
    notify: &Notify,
) -> Option<Order> {
    let order = snapshot.orders.iter_mut().find(|order| order.id == id)?;
    order.notify.remove(notify);
    Some(order.clone())
}

pub fn set_flavor_config(
    snapshot: &mut OrderSystemSnapshot,
    flavor: Flavor,
    config: FlavorConfig,
) -> StatusUpdate {
    snapshot.flavor_configs[flavor] = config;
    order_status::update_order_statuses(snapshot)
}

fn empty_status_update() -> StatusUpdate {
    StatusUpdate {
        newly_ready_orders: Vec::new(),
        notifications: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use crate::domain::snapshot::{Flavor, Item, Order, OrderStatus, OrderSystemSnapshot};

    use super::{cancel_order, complete_order, create_order, update_order_priority};

    fn item() -> Item {
        Item {
            flavor: Flavor::Tsubuan,
            quantity: 1,
        }
    }

    fn waiting_order(id: u32) -> Order {
        Order {
            id,
            items: vec![item()],
            status: OrderStatus::Waiting,
            ordered_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
                + Duration::seconds(id.into()),
            ready_at: None,
            completed_at: None,
            notify: Default::default(),
            is_priority: false,
        }
    }

    #[test]
    fn create_order_allocates_next_internal_id() {
        let mut snapshot = OrderSystemSnapshot {
            orders: vec![waiting_order(41)],
            ..Default::default()
        };

        let mutation = create_order(&mut snapshot, vec![item()], false);

        assert_eq!(mutation.result.id, 42);
    }

    #[test]
    fn cancelling_ready_order_returns_stock() {
        let mut ready = waiting_order(1);
        ready.status = OrderStatus::Ready;
        let mut snapshot = OrderSystemSnapshot {
            orders: vec![ready],
            ..Default::default()
        };

        let mutation = cancel_order(&mut snapshot, 1).expect("order exists");

        assert_eq!(mutation.result.status, OrderStatus::Cancelled);
        assert_eq!(snapshot.unallocated_stock[Flavor::Tsubuan], 1);
    }

    #[test]
    fn completing_unknown_order_returns_none() {
        let mut snapshot = OrderSystemSnapshot::default();

        assert!(complete_order(&mut snapshot, 1).is_none());
    }

    #[test]
    fn priority_update_recalculates_ready_order() {
        let mut snapshot = OrderSystemSnapshot {
            orders: vec![waiting_order(1), waiting_order(2)],
            ..Default::default()
        };
        snapshot.unallocated_stock[Flavor::Tsubuan] = 1;

        let mutation = update_order_priority(&mut snapshot, 2, true).expect("order exists");

        assert!(mutation.result.is_priority);
        assert_eq!(mutation.status_update.newly_ready_orders, vec![2]);
    }
}
