pub mod connector;
pub mod connectors;
pub mod transformation;

use std::collections::HashSet;

pub use connector::{Connector, ConnectorMeta};
pub use connectors::csv::{CsvConfig, CsvError, CsvSource};
pub use connectors::websocket::{WebSocketConfig, WebSocketError, WebSocketSource};

/// Returns metadata for all registered connectors.
pub fn get_available_connectors(
) -> Vec<(&'static str, serde_json::Value, serde_json::Value, &'static str, &'static str)> {
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
        ),
    ]
}

/// Returns the set of available connector type identifiers.
pub fn get_available_connector_types() -> HashSet<&'static str> {
    [CsvSource::connector_type(), WebSocketSource::connector_type()]
        .into_iter()
        .collect()
}

/// Returns `true` if `connector_type` names a registered connector.
pub fn is_connector_available(connector_type: &str) -> bool {
    get_available_connector_types().contains(connector_type)
}
