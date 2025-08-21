use std::{fs, path::Path};

use duckdb::Connection;

use anyhow::Result;

use crate::models::Pipeline;



pub struct Database {
    db_path: String
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
        let mut query = conn.prepare(
            "SELECT * FROM pipelines ORDER BY id"
        )?;
        let rows = query.query_map([], |row| {
            Ok(Pipeline {
                id: row.get("id")?,
                name: row.get("name")?,
                nodes: vec![],
                edges: vec![],
            })
        })?;

        rows.collect::<Result<Vec<Pipeline>, _>>().map_err(|e| anyhow::Error::from(e))
    }

    pub fn add_pipeline(&self, pipeline: &Pipeline) -> Result<i32> {
        let conn = Connection::open(&self.db_path)?;

        let mut query = conn.prepare(
            "INSERT INTO pipelines (name) VALUES (?1) RETURNING id",
        )?;

        let id: i32 = query.query_row([&pipeline.name], |row| {
            row.get(0)
        })?;
        Ok(id)
    }
}