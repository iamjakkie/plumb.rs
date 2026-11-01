use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::transformation::TransformationMeta;

/// Metadata-only destination; the engine emits preview events itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    /// Emit every Nth record (1 = all).
    #[serde(default = "default_sample")]
    pub sample_every: u64,
}

fn default_sample() -> u64 {
    1
}

pub struct PreviewMeta;

impl TransformationMeta for PreviewMeta {
    fn operator_type() -> &'static str {
        "preview"
    }

    fn config_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "sample_every": {
                    "type": "integer",
                    "default": 1,
                    "description": "Emit every Nth record (1 = all)"
                }
            }
        })
    }

    fn display_name() -> &'static str {
        "Preview"
    }

    fn description() -> &'static str {
        "Stream records to the live events panel"
    }
}
