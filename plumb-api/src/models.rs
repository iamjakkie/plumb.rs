use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Pipeline {
    pub id: i32,
    pub name: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>
}

impl Pipeline {
    pub fn new(name: String) -> Self {
        Self {
            id: 0,
            name,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}


#[derive(Serialize, Deserialize)]
pub enum NodeType {
    Connector {
        max_outputs: Option<usize>
    },
    Transformation {
        min_inputs: usize,
        max_inputs: Option<usize>,
        min_outputs: usize,
        max_outputs: Option<usize>
    },
    Destination
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    id: i32,
    node_type: NodeType,
    name: String,
    config: serde_json::Value
}

#[derive(Serialize, Deserialize)]
pub struct Edge {
    from: i32,
    to: i32,
}

#[derive(Serialize, Deserialize)]
pub struct NodeState {
    id: i32,
    status: String,
    description: String,
    error: Option<String>,
    last_activity: Option<String>,
    metrics: Option<serde_json::Value>
}