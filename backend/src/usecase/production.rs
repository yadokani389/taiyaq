use crate::domain::order_status::{self, StatusUpdate};
use crate::domain::snapshot::{Item, OrderSystemSnapshot};

pub fn register_completed_production(
    snapshot: &mut OrderSystemSnapshot,
    production: Vec<Item>,
) -> StatusUpdate {
    for item in production {
        snapshot.unallocated_stock[item.flavor] += item.quantity;
    }

    order_status::update_order_statuses(snapshot)
}
