#[async_trait]
pub trait Transformation {
    type Config;
    type Error;

    async fn new(config: Self::Config) -> Result<Self, Self::Error>
    where 
        Self: Sized;

    async fn transform(&mut self, inputs: HashMap<String, Vec<serde_json::Value>>) -> Result<HashMap<String, Vec<serde_json::Value>>, Self::Error>;

}