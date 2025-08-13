use async_trait::async_trait;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::task;

use crate::source::Source;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvConfig {
    pub file_path: String,
    pub has_headers: bool,
    pub delimiter: Option<char>, // Default to comma
}

#[derive(Debug, Error)]
pub enum CsvError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("End of file reached")]
    EndOfFile,
}

pub struct CsvSource {
    reader: Arc<Mutex<csv::Reader<BufReader<File>>>>,
    headers: Option<Vec<String>>,
    current_row: u64,
}

#[async_trait]
impl Source for CsvSource {
    type Config = CsvConfig;
    type Item = Vec<u8>;
    type Error = CsvError;

    async fn connect(config: Self::Config) -> Result<Self, Self::Error> {
        // Run file operations in a blocking task since CSV reader is sync
        let result = task::spawn_blocking(move || {
            let file = File::open(&config.file_path)?;
            let buf_reader = BufReader::new(file);
            
            let mut reader = ReaderBuilder::new()
                .delimiter(config.delimiter.unwrap_or(',') as u8)
                .has_headers(config.has_headers)
                .from_reader(buf_reader);

            let headers = if config.has_headers {
                let headers = reader.headers()?.clone();
                Some(headers.iter().map(|h| h.to_string()).collect())
            } else {
                None
            };

            Ok::<_, CsvError>(CsvSource {
                reader: Arc::new(Mutex::new(reader)),
                headers,
                current_row: 0,
            })
        }).await.map_err(|_| CsvError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Task join error"
        )))??;

        Ok(result)
    }

    async fn next(&mut self) -> Option<Result<Self::Item, Self::Error>> {
        let headers = self.headers.clone();
        let mut record = csv::StringRecord::new();
        
        let reader_clone = Arc::clone(&self.reader);
        let read_result = task::spawn_blocking(move || {
            let mut reader = reader_clone.lock().unwrap();
            let mut record = csv::StringRecord::new();
            match reader.read_record(&mut record) {
                Ok(true) => {
                    let fields: Vec<String> = record.iter().map(|s| s.to_string()).collect();
                    Ok(fields)
                }
                Ok(false) => Err(CsvError::EndOfFile),
                Err(e) => Err(CsvError::Csv(e))
            }
        }).await;

        match read_result {
            Ok(Ok(fields)) => {
                self.current_row += 1;
                
                let json_obj = if let Some(ref headers) = headers {
                    // Create JSON object using headers as keys
                    let mut obj = serde_json::Map::new();
                    for (i, field) in fields.iter().enumerate() {
                        let key = headers.get(i).cloned().unwrap_or_else(|| format!("col_{}", i));
                        obj.insert(key, json!(field));
                    }
                    json!(obj)
                } else {
                    // No headers, create array
                    json!(fields)
                };

                match serde_json::to_vec(&json_obj) {
                    Ok(bytes) => Some(Ok(bytes)),
                    Err(e) => Some(Err(CsvError::Json(e))),
                }
            }
            Ok(Err(e)) => Some(Err(e)),
            Err(_) => Some(Err(CsvError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Task join error"
            )))),
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        // CSV reader doesn't need explicit closing
        Ok(())
    }
}
