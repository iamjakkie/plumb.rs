use std::{collections::HashMap, fs, path::Path};

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::models::{Edge, Node, NodeStatus, NodeType, Pipeline, PipelineStatus, ValidationResult};

pub struct Database {
    db_path: String,
}

impl Database {
    pub fn new() -> Result<Self> {
        let path = Path::new("./data");
        if !path.exists() {
            fs::create_dir(path)?;
        }

        let db_path = path.join("plumb.db");

        if !db_path.exists() {
            let conn = Connection::open(&db_path)?;
            let schema = include_str!("schema.sql");
            conn.execute_batch(schema)?;
        }

        Ok(Self {
            db_path: db_path.to_string_lossy().into_owned(),
        })
    }

    fn connect(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    // ------------------------------------------------------------------
    // Pipelines
    // ------------------------------------------------------------------

    pub fn get_all_pipelines(&self) -> Result<Vec<Pipeline>> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT id, name, status, created_at FROM pipelines ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
                status: row.get::<_, String>("status")?.parse().unwrap_or_default(),
                created_at: row.get("created_at")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(anyhow::Error::from)
    }

    pub fn get_pipeline(&self, pipeline_id: i32) -> Result<Pipeline> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT id, name, status, created_at FROM pipelines WHERE id = ?1")?;

        let mut pipeline = stmt.query_row([pipeline_id], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
                status: row.get::<_, String>("status")?.parse().unwrap_or_default(),
                created_at: row.get("created_at")?,
            })
        })?;

        pipeline.nodes = Self::load_nodes(&conn, pipeline_id)?;
        pipeline.edges = Self::load_edges(&conn, pipeline_id)?;
        Ok(pipeline)
    }

    pub fn add_pipeline(&self, pipeline: &Pipeline) -> Result<i32> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("INSERT INTO pipelines (name) VALUES (?1) RETURNING id")?;
        let id: i32 = stmt.query_row([&pipeline.name], |row| row.get(0))?;
        Ok(id)
    }

    pub fn remove_pipeline(&self, pipeline_id: i32) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM pipelines WHERE id = ?1", [pipeline_id])?;
        Ok(())
    }

    pub fn clone_pipeline(&self, pipeline: &Pipeline) -> Result<Pipeline> {
        let name = format!("{}_copy", pipeline.name);
        let stub = Pipeline::new(name.clone());
        let new_id = self.add_pipeline(&stub)?;

        let mut node_id_map = HashMap::new();
        for node in &pipeline.nodes {
            let new_node_id = self.add_node(new_id, node)?;
            node_id_map.insert(node.id, new_node_id);
        }

        Ok(Pipeline {
            id: new_id,
            name,
            nodes: pipeline.nodes.clone(),
            edges: pipeline.edges.clone(),
            status: crate::models::PipelineStatus::Idle,
            created_at: None,
        })
    }

    // ------------------------------------------------------------------
    // Nodes
    // ------------------------------------------------------------------

    pub fn add_node(&self, pipeline_id: i32, node: &Node) -> Result<i32> {
        let conn = self.connect()?;

        let node_type_str = node.node_type.as_str();
        let config_json = node.config_json()?;
        // Store the full NodeType enum as JSON in constraints so we can reconstruct it on load.
        let constraints_json = serde_json::to_string(&node.node_type)?;

        let mut stmt = conn.prepare(
            "INSERT INTO nodes (pipeline_id, node_type, name, config, constraints) \
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
        )?;
        let id: i32 = stmt.query_row(
            params![pipeline_id, node_type_str, &node.name, &config_json, &constraints_json],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    fn load_nodes(conn: &Connection, pipeline_id: i32) -> Result<Vec<Node>> {
        let mut stmt = conn.prepare(
            "SELECT id, pipeline_id, name, config, constraints, status \
             FROM nodes WHERE pipeline_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([pipeline_id], |row| {
            // constraints column stores the serialised NodeType for reconstruction
            let constraints_str: Option<String> = row.get("constraints")?;
            let node_type: NodeType = constraints_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(NodeType::Destination);

            let config_str: Option<String> = row.get("config")?;
            let config = config_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::json!({}));

            let status = match row
                .get::<_, String>("status")
                .unwrap_or_default()
                .as_str()
            {
                "running" => NodeStatus::Running,
                "error" => NodeStatus::Error,
                _ => NodeStatus::Idle,
            };

            Ok(Node {
                id: row.get("id")?,
                pipeline_id: row.get("pipeline_id")?,
                node_type,
                name: row.get("name")?,
                config,
                constraints: None,
                status,
                created_at: None,
                updated_at: None,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(anyhow::Error::from)
    }

    // ------------------------------------------------------------------
    // Edges
    // ------------------------------------------------------------------

    pub fn add_edge(&self, pipeline_id: i32, from_node: i32, to_node: i32) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO edges (pipeline_id, from_node, to_node) VALUES (?1, ?2, ?3)",
            params![pipeline_id, from_node, to_node],
        )?;
        Ok(())
    }

    pub fn update_pipeline_status(&self, pipeline_id: i32, status: PipelineStatus) -> Result<()> {
        let conn = self.connect()?;
        let status_str = match status {
            PipelineStatus::Running => "running",
            PipelineStatus::Error => "error",
            PipelineStatus::Idle => "idle",
        };
        conn.execute(
            "UPDATE pipelines SET status = ?1 WHERE id = ?2",
            rusqlite::params![status_str, pipeline_id],
        )?;
        Ok(())
    }

    pub fn validate_pipeline(&self, pipeline_id: i32) -> Result<ValidationResult> {
        let pipeline = self.get_pipeline(pipeline_id)?;
        let mut errors: Vec<String> = Vec::new();

        if pipeline.nodes.is_empty() {
            errors.push("Pipeline has no nodes.".into());
            return Ok(ValidationResult { valid: false, errors });
        }

        let has_connector = pipeline.nodes.iter().any(|n| n.node_type.as_str() == "connector");
        let has_destination = pipeline.nodes.iter().any(|n| n.node_type.as_str() == "destination");

        if !has_connector {
            errors.push("Pipeline needs at least one Connector node.".into());
        }
        if !has_destination {
            errors.push("Pipeline needs at least one Destination node.".into());
        }

        // Every node must have at least one edge if the pipeline has more than one node
        if pipeline.nodes.len() > 1 {
            let connected: std::collections::HashSet<i32> = pipeline
                .edges
                .iter()
                .flat_map(|e| [e.from, e.to])
                .collect();
            for node in &pipeline.nodes {
                if !connected.contains(&node.id) {
                    errors.push(format!("Node '{}' is not connected to any other node.", node.name));
                }
            }
        }

        // Connectors must have at least one outgoing edge
        for node in pipeline.nodes.iter().filter(|n| n.node_type.as_str() == "connector") {
            if !pipeline.edges.iter().any(|e| e.from == node.id) {
                errors.push(format!("Connector '{}' has no outgoing connection.", node.name));
            }
        }

        // Destinations must have at least one incoming edge
        for node in pipeline.nodes.iter().filter(|n| n.node_type.as_str() == "destination") {
            if !pipeline.edges.iter().any(|e| e.to == node.id) {
                errors.push(format!("Destination '{}' has no incoming connection.", node.name));
            }
        }

        Ok(ValidationResult { valid: errors.is_empty(), errors })
    }

    fn load_edges(conn: &Connection, pipeline_id: i32) -> Result<Vec<Edge>> {
        let mut stmt = conn
            .prepare("SELECT from_node, to_node FROM edges WHERE pipeline_id = ?1")?;
        let rows = stmt.query_map([pipeline_id], |row| {
            Ok(Edge { from: row.get("from_node")?, to: row.get("to_node")? })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(anyhow::Error::from)
    }
}
