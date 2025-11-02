use enum_map::EnumMap;
use std::fs;
use strum::IntoEnumIterator;
use taiyaq_backend::data::{Data, Flavor, OrderStatus};

fn main() -> anyhow::Result<()> {
    let data_str = fs::read_to_string("data.json")?;
    let data: Data = serde_json::from_str(&data_str)?;

    let mut flavor_counts = EnumMap::from_fn(|_| 0);

    data.orders
        .iter()
        .filter(|o| o.status == OrderStatus::Completed)
        .flat_map(|o| &o.items)
        .for_each(|item| {
            flavor_counts[item.flavor] += item.quantity;
        });

    let sum: usize = flavor_counts.iter().map(|(_, n)| n).sum();

    println!("Total completed taiyaki sold by flavor:");
    for flavor in Flavor::iter() {
        println!("- {}: {}", flavor, flavor_counts[flavor]);
    }

    println!("sum: {sum}");

    Ok(())
}
