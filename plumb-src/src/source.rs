use async_trait::async_trait;

#[async_trait]
pub trait Source {
    type Config;
    type Item;
    type Error;

    async fn connect(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;
    async fn next(&mut self) -> Option<Result<Self::Item, Self::Error>>;
    async fn close(&mut self) -> Result<(), Self::Error>;
}