use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::transformation::{TransformError, Transformation, TransformationMeta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    /// Comma-separated field names to keep. Empty = keep all, then apply renames.
    #[serde(default)]
    pub fields: Option<String>,
    /// Rename pairs as `old:new,old2:new2`.
    #[serde(default)]
    pub rename: Option<String>,
}

pub struct MapTransform {
    fields: Option<Vec<String>>,
    rename: Vec<(String, String)>,
}

fn parse_fields(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn parse_rename(s: &str) -> Result<Vec<(String, String)>, TransformError> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (from, to) = part.split_once(':').ok_or_else(|| {
            TransformError::Config(format!("rename entry '{part}' must be old:new"))
        })?;
        out.push((from.trim().to_string(), to.trim().to_string()));
    }
    Ok(out)
}

#[async_trait]
impl Transformation for MapTransform {
    type Config = MapConfig;
    type Error = TransformError;

    async fn new(config: Self::Config) -> Result<Self, Self::Error> {
        let fields = config
            .fields
            .as_deref()
            .map(parse_fields)
            .filter(|v| !v.is_empty());
        let rename = match config.rename.as_deref() {
            Some(s) if !s.trim().is_empty() => parse_rename(s)?,
            _ => Vec::new(),
        };
        Ok(Self { fields, rename })
    }

    async fn apply(&mut self, record: Value) -> Result<Option<Value>, Self::Error> {
        let Some(obj) = record.as_object() else {
            return Ok(Some(record));
        };

        let mut next = Map::new();
        if let Some(fields) = &self.fields {
            for f in fields {
                if let Some(v) = obj.get(f) {
                    next.insert(f.clone(), v.clone());
                }
            }
        } else {
            next = obj.clone();
        }

        for (from, to) in &self.rename {
            if let Some(v) = next.remove(from) {
                next.insert(to.clone(), v);
            }
        }

        Ok(Some(Value::Object(next)))
    }
}

impl TransformationMeta for MapTransform {
    fn operator_type() -> &'static str {
        "map"
    }

    fn config_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "fields": {
                    "type": "string",
                    "description": "Comma-separated fields to keep (empty = all)"
                },
                "rename": {
                    "type": "string",
                    "description": "Rename pairs as old:new,old2:new2"
                }
            }
        })
    }

    fn display_name() -> &'static str {
        "Map"
    }

    fn description() -> &'static str {
        "Project and/or rename JSON fields"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn map_project_and_rename() {
        let mut t = MapTransform::new(MapConfig {
            fields: Some("a,b".into()),
            rename: Some("a:alpha".into()),
        })
        .await
        .unwrap();
        let out = t
            .apply(json!({"a": 1, "b": 2, "c": 3}))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(out, json!({"alpha": 1, "b": 2}));
    }
}
