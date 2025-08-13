use plumb_src::{Source, WebSocketSource, WebSocketConfig};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Binance WebSocket connection...");

    let subscription_msg = json!({
        "method": "SUBSCRIBE",
        "params": ["bnbusdt@trade", "ethusdt@trade"],
        "id": 1
    }).to_string();

    let config = WebSocketConfig {
        url: "wss://fstream.binance.com/stream".to_string(),
        subscription_message: Some(subscription_msg),
        headers: None,
    };

    println!("Connecting to: {}", config.url);
    println!("Subscription message: {}", config.subscription_message.as_ref().unwrap());

    let mut source = WebSocketSource::connect(config).await?;
    println!("Connected successfully!");

    // Read first 5 messages
    for i in 0..5 {
        match source.next().await {
            Some(Ok(data)) => {
                let message = String::from_utf8_lossy(&data);
                println!("Message {}: {}", i + 1, message);
            }
            Some(Err(e)) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
            None => {
                println!("Connection closed");
                break;
            }
        }
    }

    source.close().await?;
    println!("Connection closed successfully");

    Ok(())
}
