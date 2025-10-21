mod api;
mod data;

use crate::data::{AppRegistry, Data};
use api::route::routes;
use dotenvy::dotenv;
use std::{env, net::SocketAddr};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let line_token: String =
        env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("Failed getting LINE_CHANNEL_ACCESS_TOKEN");

    let registry = AppRegistry::new(Data::new(line_token));

    let app = routes().with_state(registry);

    let addr = SocketAddr::from(([127, 0, 0, 1], 38000));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
