use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

/// Single-record transform used by the local engine.
#[async_trait]
pub trait Transformation: Send {
    type Config: Send;
    type Error: std::error::Error + Send + Sync + 'static;

    async fn new(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Return `Some(record)` to forward, or `None` to drop.
    async fn apply(&mut self, record: Value) -> Result<Option<Value>, Self::Error>;
}

pub trait TransformationMeta {
    fn operator_type() -> &'static str;
    fn config_schema() -> Value;
    fn display_name() -> &'static str;
    fn description() -> &'static str;
}

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("invalid config: {0}")]
    Config(String),
    #[error("{0}")]
    Other(String),
}
