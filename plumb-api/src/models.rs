use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Pipeline {
    pub id: i32,
    pub name: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub status: String,
    pub created_at: Option<String>,
}

impl Pipeline {
    pub fn new(name: String) -> Self {
        Self {
            id: 0,
            name,
            nodes: Vec::new(),
            edges: Vec::new(),
            status: "idle".to_string(),
            created_at: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Node {
    pub id: i32,
    pub pipeline_id: i32,
    pub node_type: NodeType,
    pub name: String,
    pub config: serde_json::Value,
    pub constraints: Option<serde_json::Value>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

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
            status: "idle".to_string(),
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
        Self::new(
            pipeline_id,
            name,
            NodeType::Connector { max_outputs },
            config,
            constraints,
        )
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
            NodeType::Transformation {
                min_inputs,
                max_inputs,
                min_outputs,
                max_outputs,
            },
            config,
            None,
        )
    }

    pub fn destination(pipeline_id: i32, name: String, config: serde_json::Value) -> Self {
        Self::new(pipeline_id, name, NodeType::Destination, config, None)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Edge {
    from: i32,
    to: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConnectorType {
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "websocket")]
    WebSocket,
    #[serde(rename = "rest")]
    Rest,
    #[serde(rename = "kafka")]
    Kafka,
}

impl ConnectorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorType::Csv => "csv",
            ConnectorType::WebSocket => "websocket",
            ConnectorType::Rest => "rest",
            ConnectorType::Kafka => "kafka",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "csv" => Ok(ConnectorType::Csv),
            "websocket" => Ok(ConnectorType::WebSocket),
            
        }
    }
}