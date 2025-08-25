use std::{any::Any, collections::HashMap, fs, path::Path};

use duckdb::{params, Connection};

use anyhow::Result;

use crate::models::{Node, NodeType, Pipeline};

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
            db_path: db_path.to_string_lossy().to_string(),
        })
    }

    pub fn get_all_pipelines(&self) -> Result<Vec<Pipeline>> {
        let conn = Connection::open(&self.db_path)?;
        let mut query = conn.prepare("SELECT * FROM pipelines ORDER BY id")?;
        let rows = query.query_map([], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
            })
        })?;

        rows.collect::<Result<Vec<Pipeline>, _>>()
            .map_err(|e| anyhow::Error::from(e))
    }

    fn get_pipeline(&self, pipeline_id: i32) -> Result<Pipeline> {
        let conn = Connection::open(&self.db_path)?;

        let mut query = conn.prepare("SELECT * FROM pipelines WHERE id = ?1")?;

        let pipeline = query.query_row([&pipeline_id], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
            })
        })?;

        Ok(pipeline)
    }

    pub fn add_pipeline(&self, pipeline: &Pipeline) -> Result<i32> {
        let conn = Connection::open(&self.db_path)?;

        let mut query = conn.prepare("INSERT INTO pipelines (name) VALUES (?1) RETURNING id")?;

        let id: i32 = query.query_row([&pipeline.name], |row| row.get(0))?;
        Ok(id)
    }

    fn add_node_to_pipeline(&self, pipeline_id: i32, node: &Node) -> Result<i32> {
        let conn = Connection::open(&self.db_path)?;

        if self.get_pipeline(pipeline_id).is_err() {
            return Err(anyhow::anyhow!(
                "Can't add node to pipeline that does not exist."
            ));
        }

        let mut query = conn.prepare(
            "INSERT INTO nodes (pipeline_id, node_type, name, config, constraints) VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
        )?;

        let node_type = match node.node_type {
            NodeType::Connector { .. } => "connector",
            NodeType::Transformation { .. } => "transformation",
            NodeType::Destination => "desination",
        };

        let config_json = serde_json::to_string(&node.config)?;
        let constraints_json = serde_json::to_string(&node.node_type)?;

        let id: i32 = query.query_row(
            params![
                &pipeline_id,
                node_type,
                &node.name,
                &config_json,
                &constraints_json,
            ],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    pub fn clone_pipeline(&self, pipeline: &Pipeline) -> Result<Pipeline> {
        let name = pipeline.name.clone() + "_copy";
        let new_pipeline = Pipeline::new(name.clone());
        let id = self.add_pipeline(&new_pipeline)?;

        let mut node_id_map = HashMap::new();
        for node in &pipeline.nodes {
            let new_node_id = self.add_node_to_pipeline(id, node)?;
            node_id_map.insert(node.id, new_node_id);
        }
        Ok(Pipeline {
            id,
            name: name,
            nodes: pipeline.nodes.clone(),
            edges: pipeline.edges.clone(),
        })
    }
}
