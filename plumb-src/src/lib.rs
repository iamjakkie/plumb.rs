pub mod connector;
pub mod connectors;

use std::collections::HashSet;

pub use connectors::websocket::{WebSocketSource, WebSocketConfig, WebSocketError};
pub use connectors::csv::{CsvSource, CsvConfig, CsvError};

use crate::connector::ConnectorMeta;

/// Returns a list of available connectors.
pub fn get_available_connectors() -> Vec<(&'static str, serde_json::Value, serde_json::Value, &'static str, &'static str)> {
    vec![
        (
            CsvSource::connector_type(),
            CsvSource::config_schema(),
            CsvSource::constraint_schema(),
            CsvSource::display_name(),
            CsvSource::description(),
        ),
        (
            WebSocketSource::connector_type(),
            WebSocketSource::config_schema(),
            WebSocketSource::constraint_schema(),
            WebSocketSource::display_name(),
            WebSocketSource::description(),
        )
    ]
}

/// Returns a HashSet of available connector types for efficient lookup.
pub fn get_available_connector_types() -> HashSet<&'static str> {
    let mut types = HashSet::new();
    types.insert(CsvSource::connector_type());
    types.insert(WebSocketSource::connector_type());
    types
}

/// Check if a connector type is available.
pub fn is_connector_available(connector_type: &str) -> bool {
    get_available_connector_types().contains(connector_type)
}