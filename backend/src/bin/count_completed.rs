use enum_map::EnumMap;
use strum::IntoEnumIterator;
use taiyaq_backend::data::{Flavor, OrderStatus};
use taiyaq_backend::storage::{self, SqliteRepository};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/taiyaq.sqlite".to_string());
    let repository = SqliteRepository::new(storage::connect(&database_url).await?);
    let data = repository.load().await?;

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
