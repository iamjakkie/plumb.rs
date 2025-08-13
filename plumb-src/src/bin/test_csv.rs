use plumb_src::{Source, CsvSource, CsvConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing CSV source connection...");

    let config = CsvConfig {
        file_path: "test_data.csv".to_string(),
        has_headers: true,
        delimiter: None, // Use default comma
    };

    println!("Reading from: {}", config.file_path);
    println!("Has headers: {}", config.has_headers);

    let mut source = CsvSource::connect(config).await?;
    println!("CSV source connected successfully!");

    // Read all records
    let mut count = 0;
    while let Some(result) = source.next().await {
        match result {
            Ok(data) => {
                let message = String::from_utf8_lossy(&data);
                count += 1;
                println!("Record {}: {}", count, message);
            }
            Err(e) => {
                eprintln!("Error reading record: {}", e);
                break;
            }
        }
    }

    println!("Finished reading {} records", count);
    source.close().await?;
    println!("CSV source closed successfully");

    Ok(())
}
