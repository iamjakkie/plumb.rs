use std::{collections::HashMap, env, fs, path::Path};

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};

use crate::models::{Edge, Node, NodeStatus, NodeType, Pipeline, PipelineStatus, ValidationResult};

pub struct Database {
    db_path: String,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "./data/plumb.db".into());
        let path = Path::new(&db_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let is_new = !path.exists();
        let conn = Connection::open(path)?;
        if is_new {
            conn.execute_batch(include_str!("schema.sql"))?;
        }
        Self::migrate(&conn)?;

        Ok(Self {
            db_path: path.to_string_lossy().into_owned(),
        })
    }

    fn connect(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    /// Patch older DBs created before columns existed. SQLite does not alter
    /// `CREATE TABLE IF NOT EXISTS`, so existing files need explicit ADDs.
    fn migrate(conn: &Connection) -> Result<()> {
        let pipeline_cols = Self::table_columns(conn, "pipelines")?;
        if !pipeline_cols.iter().any(|c| c == "status") {
            conn.execute(
                "ALTER TABLE pipelines ADD COLUMN status TEXT DEFAULT 'idle'",
                [],
            )?;
        }

        let node_cols = Self::table_columns(conn, "nodes")?;
        if !node_cols.iter().any(|c| c == "status") {
            conn.execute(
                "ALTER TABLE nodes ADD COLUMN status TEXT DEFAULT 'idle'",
                [],
            )?;
        }
        if !node_cols.iter().any(|c| c == "last_updated") {
            conn.execute(
                "ALTER TABLE nodes ADD COLUMN last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP",
                [],
            )?;
        }

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS node_states (
                node_id INTEGER REFERENCES nodes(id),
                status TEXT NOT NULL,
                last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (node_id)
            );",
        )?;
        Ok(())
    }

    fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let cols = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(cols)
    }

    // ------------------------------------------------------------------
    // Pipelines
    // ------------------------------------------------------------------

    pub fn get_all_pipelines(&self) -> Result<Vec<Pipeline>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.status, p.created_at, \
             (SELECT COUNT(*) FROM nodes n WHERE n.pipeline_id = p.id) AS node_count \
             FROM pipelines p ORDER BY p.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
                status: row.get::<_, String>("status")?.parse().unwrap_or_default(),
                created_at: row.get("created_at")?,
                node_count: row.get("node_count")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)
    }

    pub fn get_pipeline(&self, pipeline_id: i32) -> Result<Pipeline> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT id, name, status, created_at FROM pipelines WHERE id = ?1")?;

        let mut pipeline = stmt
            .query_row([pipeline_id], |row| {
                Ok(Pipeline {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    nodes: vec![],
                    edges: vec![],
                    status: row.get::<_, String>("status")?.parse().unwrap_or_default(),
                    created_at: row.get("created_at")?,
                    node_count: 0,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => anyhow!("pipeline {pipeline_id} not found"),
                other => anyhow::Error::from(other),
            })?;

        pipeline.nodes = Self::load_nodes(&conn, pipeline_id)?;
        pipeline.edges = Self::load_edges(&conn, pipeline_id)?;
        pipeline.node_count = pipeline.nodes.len() as i32;
        Ok(pipeline)
    }

    pub fn add_pipeline(&self, pipeline: &Pipeline) -> Result<i32> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("INSERT INTO pipelines (name) VALUES (?1) RETURNING id")?;
        let id: i32 = stmt.query_row([&pipeline.name], |row| row.get(0))?;
        Ok(id)
    }

    pub fn remove_pipeline(&self, pipeline_id: i32) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM edges WHERE pipeline_id = ?1", [pipeline_id])?;
        conn.execute("DELETE FROM nodes WHERE pipeline_id = ?1", [pipeline_id])?;
        conn.execute("DELETE FROM pipelines WHERE id = ?1", [pipeline_id])?;
        Ok(())
    }

    pub fn clone_pipeline(&self, pipeline: &Pipeline) -> Result<Pipeline> {
        let name = format!("{}_copy", pipeline.name);
        let stub = Pipeline::new(name.clone());
        let new_id = self.add_pipeline(&stub)?;

        let mut node_id_map = HashMap::new();
        let mut new_nodes = Vec::new();
        for node in &pipeline.nodes {
            let new_node_id = self.add_node(new_id, node)?;
            node_id_map.insert(node.id, new_node_id);
            new_nodes.push(Node {
                id: new_node_id,
                pipeline_id: new_id,
                ..node.clone()
            });
        }

        let mut new_edges = Vec::new();
        for edge in &pipeline.edges {
            let Some(&from) = node_id_map.get(&edge.from) else {
                continue;
            };
            let Some(&to) = node_id_map.get(&edge.to) else {
                continue;
            };
            self.add_edge(new_id, from, to)?;
            new_edges.push(Edge { from, to });
        }

        let node_count = new_nodes.len() as i32;
        Ok(Pipeline {
            id: new_id,
            name,
            nodes: new_nodes,
            edges: new_edges,
            status: PipelineStatus::Idle,
            created_at: None,
            node_count,
        })
    }

    // ------------------------------------------------------------------
    // Nodes
    // ------------------------------------------------------------------

    pub fn add_node(&self, pipeline_id: i32, node: &Node) -> Result<i32> {
        let conn = self.connect()?;

        let node_type_str = node.node_type.as_str();
        let config_json = node.config_json()?;
        let constraints_json = serde_json::to_string(&node.node_type)?;

        let mut stmt = conn.prepare(
            "INSERT INTO nodes (pipeline_id, node_type, name, config, constraints) \
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
        )?;
        let id: i32 = stmt.query_row(
            params![
                pipeline_id,
                node_type_str,
                &node.name,
                &config_json,
                &constraints_json
            ],
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)
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
            return Ok(ValidationResult {
                valid: false,
                errors,
            });
        }

        let has_connector = pipeline
            .nodes
            .iter()
            .any(|n| n.node_type.as_str() == "connector");
        let has_destination = pipeline
            .nodes
            .iter()
            .any(|n| n.node_type.as_str() == "destination");

        if !has_connector {
            errors.push("Pipeline needs at least one Connector node.".into());
        }
        if !has_destination {
            errors.push("Pipeline needs at least one Destination node.".into());
        }

        let connector_count = pipeline
            .nodes
            .iter()
            .filter(|n| n.node_type.as_str() == "connector")
            .count();
        if connector_count > 1 {
            errors.push("MVP supports exactly one Connector source.".into());
        }

        for node in &pipeline.nodes {
            let typ = node
                .config
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match node.node_type.as_str() {
                "connector" if typ.is_empty() => {
                    errors.push(format!(
                        "Connector '{}' is missing config.type (csv|websocket).",
                        node.name
                    ));
                }
                "transformation" if typ.is_empty() => {
                    errors.push(format!(
                        "Transformation '{}' is missing config.type.",
                        node.name
                    ));
                }
                "destination" if typ.is_empty() => {
                    errors.push(format!(
                        "Destination '{}' is missing config.type (use preview).",
                        node.name
                    ));
                }
                _ => {}
            }
        }

        if pipeline.nodes.len() > 1 {
            let connected: std::collections::HashSet<i32> = pipeline
                .edges
                .iter()
                .flat_map(|e| [e.from, e.to])
                .collect();
            for node in &pipeline.nodes {
                if !connected.contains(&node.id) {
                    errors.push(format!(
                        "Node '{}' is not connected to any other node.",
                        node.name
                    ));
                }
            }
        }

        for node in pipeline
            .nodes
            .iter()
            .filter(|n| n.node_type.as_str() == "connector")
        {
            if !pipeline.edges.iter().any(|e| e.from == node.id) {
                errors.push(format!(
                    "Connector '{}' has no outgoing connection.",
                    node.name
                ));
            }
        }

        for node in pipeline
            .nodes
            .iter()
            .filter(|n| n.node_type.as_str() == "destination")
        {
            if !pipeline.edges.iter().any(|e| e.to == node.id) {
                errors.push(format!(
                    "Destination '{}' has no incoming connection.",
                    node.name
                ));
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
        })
    }

    fn load_edges(conn: &Connection, pipeline_id: i32) -> Result<Vec<Edge>> {
        let mut stmt =
            conn.prepare("SELECT from_node, to_node FROM edges WHERE pipeline_id = ?1")?;
        let rows = stmt.query_map([pipeline_id], |row| {
            Ok(Edge {
                from: row.get("from_node")?,
                to: row.get("to_node")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)
    }
}
