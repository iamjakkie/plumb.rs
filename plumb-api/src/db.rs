use std::{fs, path::Path};

use duckdb::Connection;

use anyhow::Result;



pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let path = Path::new("./data");
        if !path.exists() {
            fs::create_dir(path)?;
        }

        let db_path = path.join("/plumb.db");

        let connection = Connection::open(db_path)?;

        let db = Self {
            conn: connection
        };

        db.init_tables();

        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let schema = include_str!("schema.sql");
        self.conn.execute_batch(schema)?;
        Ok(())
    }
}