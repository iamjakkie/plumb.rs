use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;

use crate::connector::{Connector, ConnectorMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: String,
    pub subscription_message: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
}

#[derive(Debug, Error)]
pub enum WebSocketError {
    #[error("Connection error: {0}")]
    Connection(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Connection closed")]
    Closed,
}

pub struct WebSocketSource {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    #[allow(dead_code)] // retained for future reconnection logic
    config: WebSocketConfig,
}

#[async_trait]
impl Connector for WebSocketSource {
    type Config = WebSocketConfig;
    type Item = Vec<u8>;
    type Error = WebSocketError;

    async fn connect(config: Self::Config) -> Result<Self, Self::Error> {
        let (ws_stream, _) = connect_async(&config.url).await?;
        
        let mut source = Self {
            ws_stream,
            config: config.clone(),
        };

        // Send subscription message if provided
        if let Some(sub_msg) = &config.subscription_message {
            source.ws_stream.send(Message::Text(sub_msg.clone().into())).await?;
        }

        Ok(source)
    }

    async fn next(&mut self) -> Option<Result<Self::Item, Self::Error>> {
        match self.ws_stream.next().await {
            Some(Ok(Message::Text(text))) => Some(Ok(text.bytes().collect())),
            Some(Ok(Message::Binary(data))) => Some(Ok(data.to_vec())),
            Some(Ok(Message::Close(_))) => None,
            Some(Ok(_)) => self.next().await, // Skip ping/pong frames
            Some(Err(e)) => Some(Err(WebSocketError::Connection(e))),
            None => None,
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.ws_stream.send(Message::Close(None)).await?;
        Ok(())
    }
}

impl ConnectorMeta for WebSocketSource {
    fn connector_type() -> &'static str {
        "websocket"
    }
    
    fn config_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "WebSocket URL to connect to"},
                "subscription_message": {"type": "string", "description": "Optional message to send after connection"},
                "headers": {
                    "type": "array",
                    "items": {
                        "type": "array",
                        "items": {"type": "string"},
                        "minItems": 2,
                        "maxItems": 2
                    },
                    "description": "Optional HTTP headers as [key, value] pairs"
                }
            },
            "required": ["url"]
        })
    }
    
    fn is_available() -> bool {
        true
    }
    
    fn display_name() -> &'static str {
        "WebSocket"
    }
    
    fn description() -> &'static str {
        "Connect to WebSocket streams for real-time data"
    }

    fn constraint_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "max_message_size": {"type": "number", "description": "Maximum message size in bytes"},
                "reconnect_attempts": {"type": "number", "default": 3, "description": "Number of reconnection attempts"},
                "timeout_seconds": {"type": "number", "default": 30, "description": "Connection timeout in seconds"}
            }
        })
    }
}
