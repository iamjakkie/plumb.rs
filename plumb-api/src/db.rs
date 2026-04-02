use std::{collections::HashMap, fs, path::Path};

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};

use crate::models::{Node, Pipeline};

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

    pub fn get_all_pipelines(&self) -> Result<Vec<Pipeline>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT id, name, status, created_at FROM pipelines ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
                status: row
                    .get::<_, String>("status")?
                    .parse()
                    .unwrap_or_default(),
                created_at: row.get("created_at")?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(anyhow::Error::from)
    }

    pub fn get_pipeline(&self, pipeline_id: i32) -> Result<Pipeline> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT id, name, status, created_at FROM pipelines WHERE id = ?1")?;

        let pipeline = stmt.query_row([pipeline_id], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
                status: row
                    .get::<_, String>("status")?
                    .parse()
                    .unwrap_or_default(),
                created_at: row.get("created_at")?,
            })
        })?;

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

    pub fn add_node(&self, pipeline_id: i32, node: &Node) -> Result<i32> {
        let conn = self.connect()?;

        if self.get_pipeline(pipeline_id).is_err() {
            return Err(anyhow!("Cannot add node to pipeline {pipeline_id}: pipeline does not exist"));
        }

        let node_type = node.node_type.as_str();
        let config_json = node.config_json()?;
        let constraints_json = serde_json::to_string(&node.node_type)?;

        let mut stmt = conn.prepare(
            "INSERT INTO nodes (pipeline_id, node_type, name, config, constraints) \
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
        )?;

        let id: i32 = stmt.query_row(
            params![pipeline_id, node_type, &node.name, &config_json, &constraints_json],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    pub fn clone_pipeline(&self, pipeline: &Pipeline) -> Result<Pipeline> {
        let name = format!("{}_copy", pipeline.name);
        let new_pipeline = Pipeline::new(name.clone());
        let new_id = self.add_pipeline(&new_pipeline)?;

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
}
