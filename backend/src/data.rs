use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use enum_map::{Enum, EnumMap, enum_map};
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Debug, Enum, EnumIter, Clone, PartialEq, Eq, Hash, Copy)]
#[serde(rename_all = "camelCase")]
pub enum Flavor {
    Tsubuan,
    Custard,
    Kurikinton,
}

impl fmt::Display for Flavor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Flavor::Tsubuan => write!(f, "つぶあん"),
            Flavor::Custard => write!(f, "カスタード"),
            Flavor::Kurikinton => write!(f, "栗きんとん"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct FlavorConfig {
    pub cooking_time_minutes: u32,
    pub quantity_per_batch: u32,
}

// Data holds the core business data.
#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub orders: Vec<Order>,
    pub unallocated_stock: HashMap<Flavor, usize>,
    pub flavor_configs: EnumMap<Flavor, FlavorConfig>,
}

impl Default for Data {
    fn default() -> Self {
        let flavor_configs = enum_map! {
            Flavor::Tsubuan => FlavorConfig {
                cooking_time_minutes: 15,
                quantity_per_batch: 9,
            },
            Flavor::Custard => FlavorConfig {
                cooking_time_minutes: 15,
                quantity_per_batch: 9,
            },
            Flavor::Kurikinton => FlavorConfig {
                cooking_time_minutes: 15,
                quantity_per_batch: 2,
            },
        };
        Self {
            orders: Vec::new(),
            unallocated_stock: HashMap::new(),
            flavor_configs,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: u32,
    pub items: Vec<Item>,
    pub status: OrderStatus,
    pub ordered_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notify: Vec<Notify>,
    pub is_priority: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub flavor: Flavor,
    pub quantity: usize,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, Copy)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    #[default]
    Waiting,
    Cooking,
    Ready,
    Completed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Notify {
    pub channel: NotifyChannel,
    pub target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotifyChannel {
    Discord,
    Line,
}
