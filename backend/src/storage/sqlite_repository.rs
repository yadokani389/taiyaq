use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use strum::IntoEnumIterator;

use crate::data::{Data, Flavor, FlavorConfig, Item, Notify, Order, OrderStatus};

#[derive(Clone)]
pub struct SqliteRepository {
    pool: SqlitePool,
}

impl SqliteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn load(&self) -> anyhow::Result<Data> {
        let mut data = Data::default();

        for row in sqlx::query!("SELECT flavor, unallocated_quantity FROM stock")
            .fetch_all(&self.pool)
            .await?
        {
            let flavor = Flavor::from_db_str(&required_column(row.flavor, "flavor")?)?;
            data.unallocated_stock[flavor] = row.unallocated_quantity as usize;
        }

        for row in sqlx::query!(
            "SELECT flavor, cooking_time_minutes, quantity_per_batch FROM flavor_configs",
        )
        .fetch_all(&self.pool)
        .await?
        {
            let flavor = Flavor::from_db_str(&required_column(row.flavor, "flavor")?)?;
            data.flavor_configs[flavor] = FlavorConfig {
                cooking_time_minutes: row.cooking_time_minutes as u32,
                quantity_per_batch: row.quantity_per_batch as u32,
            };
        }

        let mut items_by_order_id = HashMap::<u32, Vec<Item>>::new();
        for row in sqlx::query!("SELECT order_id, flavor, quantity FROM order_items")
            .fetch_all(&self.pool)
            .await?
        {
            let order_id = row.order_id as u32;
            let flavor = Flavor::from_db_str(&row.flavor)?;
            let quantity = row.quantity as usize;
            items_by_order_id
                .entry(order_id)
                .or_default()
                .push(Item { flavor, quantity });
        }

        let mut notifications_by_order_id = HashMap::<u32, HashSet<Notify>>::new();
        for row in sqlx::query!(
            "SELECT order_id, kind, discord_channel_id, discord_user_id, line_user_id FROM notifications",
        )
        .fetch_all(&self.pool)
        .await?
        {
            let order_id = row.order_id as u32;
            let notify = match row.kind.as_str() {
                "discord" => Notify::Discord {
                    channel_id: required_column(row.discord_channel_id, "discord_channel_id")?
                        .parse()?,
                    user_id: required_column(row.discord_user_id, "discord_user_id")?.parse()?,
                },
                "line" => Notify::Line {
                    user_id: required_column(row.line_user_id, "line_user_id")?,
                },
                kind => anyhow::bail!("invalid notification kind: {kind}"),
            };
            notifications_by_order_id
                .entry(order_id)
                .or_default()
                .insert(notify);
        }

        data.orders = sqlx::query!(
            "SELECT id, status, ordered_at, ready_at, completed_at, is_priority FROM orders ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            let id = row.id as u32;
            Ok(Order {
                id,
                items: items_by_order_id.remove(&id).unwrap_or_default(),
                status: OrderStatus::from_db_str(&row.status)?,
                ordered_at: parse_datetime(row.ordered_at)?,
                ready_at: parse_optional_datetime(row.ready_at)?,
                completed_at: parse_optional_datetime(row.completed_at)?,
                notify: notifications_by_order_id.remove(&id).unwrap_or_default(),
                is_priority: row.is_priority != 0,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(data)
    }

    pub async fn save(&self, data: &Data) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!("DELETE FROM notifications")
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM order_items")
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM orders").execute(&mut *tx).await?;
        sqlx::query!("DELETE FROM stock").execute(&mut *tx).await?;
        sqlx::query!("DELETE FROM flavor_configs")
            .execute(&mut *tx)
            .await?;

        for flavor in Flavor::iter() {
            sqlx::query!(
                "INSERT INTO stock (flavor, unallocated_quantity) VALUES (?, ?)",
                flavor.as_db_str(),
                data.unallocated_stock[flavor] as i64,
            )
            .execute(&mut *tx)
            .await?;

            let config = data.flavor_configs[flavor];
            sqlx::query!(
                "INSERT INTO flavor_configs (flavor, cooking_time_minutes, quantity_per_batch) VALUES (?, ?, ?)",
                flavor.as_db_str(),
                config.cooking_time_minutes as i64,
                config.quantity_per_batch as i64,
            )
            .execute(&mut *tx)
            .await?;
        }

        for order in &data.orders {
            sqlx::query!(
                "INSERT INTO orders (id, status, ordered_at, ready_at, completed_at, is_priority) VALUES (?, ?, ?, ?, ?, ?)",
                order.id as i64,
                order.status.as_db_str(),
                format_datetime(order.ordered_at),
                order.ready_at.map(format_datetime),
                order.completed_at.map(format_datetime),
                i64::from(order.is_priority),
            )
            .execute(&mut *tx)
            .await?;

            for item in &order.items {
                sqlx::query!(
                    "INSERT INTO order_items (order_id, flavor, quantity) VALUES (?, ?, ?)",
                    order.id as i64,
                    item.flavor.as_db_str(),
                    item.quantity as i64,
                )
                .execute(&mut *tx)
                .await?;
            }

            for notify in &order.notify {
                match notify {
                    Notify::Discord {
                        channel_id,
                        user_id,
                    } => {
                        sqlx::query!(
                            "INSERT INTO notifications (order_id, kind, discord_channel_id, discord_user_id) VALUES (?, 'discord', ?, ?)",
                            order.id as i64,
                            channel_id.to_string(),
                            user_id.to_string(),
                        )
                        .execute(&mut *tx)
                        .await?;
                    }
                    Notify::Line { user_id } => {
                        sqlx::query!(
                            "INSERT INTO notifications (order_id, kind, line_user_id) VALUES (?, 'line', ?)",
                            order.id as i64,
                            user_id,
                        )
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }
}

fn parse_datetime(value: String) -> anyhow::Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc))
}

fn parse_optional_datetime(value: Option<String>) -> anyhow::Result<Option<DateTime<Utc>>> {
    value.map(parse_datetime).transpose()
}

fn required_column(value: Option<String>, name: &str) -> anyhow::Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("{name} must not be null"))
}

fn format_datetime(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
