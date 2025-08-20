use std::{fs, path::Path};

use duckdb::Connection;

use anyhow::Result;



pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path_str: &str) -> Result<Self> {
        let path = Path::new(path_str);
        if !path.exists() {
            fs::create_dir(path)?;
        }

        let connection = Connection::open(path)?;

        let db = Self {
            conn: connection
        };

        db.init_tables();

        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        
    }
}