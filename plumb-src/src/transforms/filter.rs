use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::transformation::{TransformError, Transformation, TransformationMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub field: String,
    #[serde(default = "default_op")]
    pub op: String,
    #[serde(default)]
    pub value: Option<String>,
}

fn default_op() -> String {
    "eq".into()
}

pub struct FilterTransform {
    config: FilterConfig,
}

fn field_value<'a>(record: &'a Value, field: &str) -> Option<&'a Value> {
    if let Some(obj) = record.as_object() {
        return obj.get(field);
    }
    None
}

fn value_as_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string().trim_matches('"').to_string(),
    }
}

#[async_trait]
impl Transformation for FilterTransform {
    type Config = FilterConfig;
    type Error = TransformError;

    async fn new(config: Self::Config) -> Result<Self, Self::Error> {
        let op = config.op.as_str();
        if !matches!(op, "eq" | "ne" | "contains" | "exists") {
            return Err(TransformError::Config(format!("unknown op '{op}'")));
        }
        if op != "exists" && config.value.is_none() {
            return Err(TransformError::Config(
                "value is required for this op".into(),
            ));
        }
        Ok(Self { config })
    }

    async fn apply(&mut self, record: Value) -> Result<Option<Value>, Self::Error> {
        let got = field_value(&record, &self.config.field);
        let pass = match self.config.op.as_str() {
            "exists" => got.is_some() && !got.unwrap().is_null(),
            "eq" => {
                let expected = self.config.value.as_deref().unwrap_or("");
                got.map(|v| value_as_str(v) == expected).unwrap_or(false)
            }
            "ne" => {
                let expected = self.config.value.as_deref().unwrap_or("");
                got.map(|v| value_as_str(v) != expected).unwrap_or(true)
            }
            "contains" => {
                let expected = self.config.value.as_deref().unwrap_or("");
                got.map(|v| value_as_str(v).contains(expected))
                    .unwrap_or(false)
            }
            _ => false,
        };
        Ok(if pass { Some(record) } else { None })
    }
}

impl TransformationMeta for FilterTransform {
    fn operator_type() -> &'static str {
        "filter"
    }

    fn config_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "field": {"type": "string", "description": "JSON field name to test"},
                "op": {
                    "type": "string",
                    "default": "eq",
                    "description": "eq | ne | contains | exists"
                },
                "value": {"type": "string", "description": "Comparison value (not needed for exists)"}
            },
            "required": ["field", "op"]
        })
    }

    fn display_name() -> &'static str {
        "Filter"
    }

    fn description() -> &'static str {
        "Keep or drop records by comparing a field"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn filter_eq() {
        let mut t = FilterTransform::new(FilterConfig {
            field: "symbol".into(),
            op: "eq".into(),
            value: Some("BTC".into()),
        })
        .await
        .unwrap();
        let keep = t
            .apply(json!({"symbol": "BTC", "px": 1}))
            .await
            .unwrap();
        assert!(keep.is_some());
        let drop = t
            .apply(json!({"symbol": "ETH", "px": 1}))
            .await
            .unwrap();
        assert!(drop.is_none());
    }
}
