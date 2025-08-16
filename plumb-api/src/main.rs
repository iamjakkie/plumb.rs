use axum::{routing::get, Router};
use dotenv::dotenv;
use std::env;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    println!("Starting Plumb API server on {}:{}", host, port);

    let app = Router::new()
        .route("/he")

    Ok(())
}