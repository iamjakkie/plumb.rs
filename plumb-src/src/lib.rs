pub mod connector;
pub mod connectors;
pub mod destinations;
pub mod transformation;
pub mod transforms;

use std::collections::HashSet;

pub use connector::{Connector, ConnectorMeta};
pub use connectors::csv::{CsvConfig, CsvError, CsvSource};
pub use connectors::websocket::{WebSocketConfig, WebSocketError, WebSocketSource};
pub use destinations::preview::{PreviewConfig, PreviewMeta};
pub use transformation::{TransformError, Transformation, TransformationMeta};
pub use transforms::{
    FilterConfig, FilterTransform, MapConfig, MapTransform, RateLimitConfig, RateLimitTransform,
};

/// Returns metadata for all registered connectors.
pub fn get_available_connectors(
) -> Vec<(
    &'static str,
    serde_json::Value,
    serde_json::Value,
    &'static str,
    &'static str,
)> {
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

/// Operator metadata: (kind, type, config_schema, display_name, description)
pub fn get_available_operators(
) -> Vec<(
    &'static str,
    &'static str,
    serde_json::Value,
    &'static str,
    &'static str,
)> {
    vec![
        (
            "transformation",
            FilterTransform::operator_type(),
            FilterTransform::config_schema(),
            FilterTransform::display_name(),
            FilterTransform::description(),
        ),
        (
            "transformation",
            MapTransform::operator_type(),
            MapTransform::config_schema(),
            MapTransform::display_name(),
            MapTransform::description(),
        ),
        (
            "transformation",
            RateLimitTransform::operator_type(),
            RateLimitTransform::config_schema(),
            RateLimitTransform::display_name(),
            RateLimitTransform::description(),
        ),
        (
            "destination",
            PreviewMeta::operator_type(),
            PreviewMeta::config_schema(),
            PreviewMeta::display_name(),
            PreviewMeta::description(),
        ),
    ]
}

pub fn get_available_connector_types() -> HashSet<&'static str> {
    [CsvSource::connector_type(), WebSocketSource::connector_type()]
        .into_iter()
        .collect()
}

pub fn is_connector_available(connector_type: &str) -> bool {
    get_available_connector_types().contains(connector_type)
}
