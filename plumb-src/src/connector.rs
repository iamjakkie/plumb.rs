use async_trait::async_trait;

#[async_trait]
pub trait Connector {
    type Config;
    type Item;
    type Error;

    async fn connect(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;
    async fn next(&mut self) -> Option<Result<Self::Item, Self::Error>>;
    async fn close(&mut self) -> Result<(), Self::Error>;
}

pub trait ConnectorMeta {
    fn connector_type() -> &'static str;

    fn config_schema() -> serde_json::Value;

    fn is_available() -> bool;

    fn display_name() -> &'static str;

    fn description() -> &'static str;
}
