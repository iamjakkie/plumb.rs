pub mod connector;
pub mod websocket;
pub mod csv;

pub use core::error::Source
pub use websocket::{WebSocketSource, WebSocketConfig, WebSocketError};
pub use csv::{CsvSource, CsvConfig, CsvError};

use crate::connector::ConnectorMeta;


pub fn get_available_connectors() -> Vec<(&'static str, serde_json::Value, &'static str, &'static str)> {
    vec![
        (
            CsvSource::connector_type(),
            CsvSource::config_schema(),
            CsvSource::display_name(),
            CsvSource::description(),
        ),
        (
            WebSocketSource::connector_type(),
            WebSocketSource::config_schema(),
            WebSocketSource::display_name(),
            WebSocketSource::description(),
        )
    ]
}