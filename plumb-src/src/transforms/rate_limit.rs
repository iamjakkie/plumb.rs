use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration, Instant};

use crate::transformation::{TransformError, Transformation, TransformationMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_rps")]
    pub records_per_second: f64,
}

fn default_rps() -> f64 {
    5.0
}

pub struct RateLimitTransform {
    interval: Duration,
    next_at: Option<Instant>,
}

#[async_trait]
impl Transformation for RateLimitTransform {
    type Config = RateLimitConfig;
    type Error = TransformError;

    async fn new(config: Self::Config) -> Result<Self, Self::Error> {
        if config.records_per_second <= 0.0 {
            return Err(TransformError::Config(
                "records_per_second must be > 0".into(),
            ));
        }
        let interval = Duration::from_secs_f64(1.0 / config.records_per_second);
        Ok(Self {
            interval,
            next_at: None,
        })
    }

    async fn apply(&mut self, record: Value) -> Result<Option<Value>, Self::Error> {
        let now = Instant::now();
        if let Some(next) = self.next_at {
            if now < next {
                sleep(next - now).await;
            }
        }
        self.next_at = Some(Instant::now() + self.interval);
        Ok(Some(record))
    }
}

impl TransformationMeta for RateLimitTransform {
    fn operator_type() -> &'static str {
        "rate_limit"
    }

    fn config_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "records_per_second": {
                    "type": "number",
                    "default": 5,
                    "description": "Max records forwarded per second"
                }
            },
            "required": ["records_per_second"]
        })
    }

    fn display_name() -> &'static str {
        "Rate Limit"
    }

    fn description() -> &'static str {
        "Throttle throughput so CSV replay is watchable"
    }
}
