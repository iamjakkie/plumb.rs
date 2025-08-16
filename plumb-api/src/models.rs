
pub struct Pipeline {
    id: i32,
    name: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>
}

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


pub struct Node {
    id: i32,
    node_type: NodeType,
    name: String,
    config: serde_json::Value
}

pub struct Edge {
    from: i32,
    to: i32,
}

pub struct NodeState {
    id: i32,
    status: String,
    description: String,
    error: Option<String>,
    last_activity: Option<String>,
    metrics: Option<serde_json::Value>
}