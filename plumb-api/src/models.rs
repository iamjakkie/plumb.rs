use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    #[default]
    Idle,
    Running,
    Error,
}

impl FromStr for PipelineStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "error" => Ok(Self::Error),
            _ => Ok(Self::Idle),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    #[default]
    Idle,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: i32,
    pub name: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub status: PipelineStatus,
    pub created_at: Option<String>,
}

impl Pipeline {
    pub fn new(name: String) -> Self {
        Self {
            id: 0,
            name,
            nodes: Vec::new(),
            edges: Vec::new(),
            status: PipelineStatus::Idle,
            created_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum NodeType {
    Connector {
        max_outputs: Option<usize>,
    },
    Transformation {
        min_inputs: usize,
        max_inputs: Option<usize>,
        min_outputs: usize,
        max_outputs: Option<usize>,
    },
    Destination,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Connector { .. } => "connector",
            NodeType::Transformation { .. } => "transformation",
            NodeType::Destination => "destination",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: i32,
    pub pipeline_id: i32,
    pub node_type: NodeType,
    pub name: String,
    pub config: serde_json::Value,
    pub constraints: Option<serde_json::Value>,
    pub status: NodeStatus,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[allow(dead_code)]
impl Node {
    pub fn new(
        pipeline_id: i32,
        name: String,
        node_type: NodeType,
        config: serde_json::Value,
        constraints: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: 0,
            pipeline_id,
            node_type,
            name,
            config,
            constraints,
            status: NodeStatus::Idle,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn config_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.config)
    }

    pub fn constraints_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.constraints)
    }

    pub fn connector(
        pipeline_id: i32,
        name: String,
        max_outputs: Option<usize>,
        config: serde_json::Value,
        constraints: Option<serde_json::Value>,
    ) -> Self {
        Self::new(pipeline_id, name, NodeType::Connector { max_outputs }, config, constraints)
    }

    pub fn transformation(
        pipeline_id: i32,
        name: String,
        min_inputs: usize,
        max_inputs: Option<usize>,
        min_outputs: usize,
        max_outputs: Option<usize>,
        config: serde_json::Value,
    ) -> Self {
        Self::new(
            pipeline_id,
            name,
            NodeType::Transformation { min_inputs, max_inputs, min_outputs, max_outputs },
            config,
            None,
        )
    }

    pub fn destination(pipeline_id: i32, name: String, config: serde_json::Value) -> Self {
        Self::new(pipeline_id, name, NodeType::Destination, config, None)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Edge {
    pub from: i32,
    pub to: i32,
}
