use axum::{routing::get, Router};
use dotenv::dotenv;
use std::env;
use anyhow::Result;

mod models;

use models::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    println!("Starting Plumb API server on {}:{}", host, port);

    let app = Router::new()
        .route("/he")

    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

/*
GET    /api/pipelines              # List all pipelines
POST   /api/pipelines              # Create new pipeline
GET    /api/pipelines/{id}         # Get pipeline details
PUT    /api/pipelines/{id}         # Update pipeline
DELETE /api/pipelines/{id}         # Delete pipeline
*/

async fn list_pipelines() -> Json<Vec<Pipeline>> {

}

async fn create_pipeline(pipeline: Pipeline) -> Result<()> {

}

async fn get_pipeline(id: i32) -> Json<Pipeline> {

}

async fn update_pipeline(pipeline: Pipeline) -> Result<()> {

}

async fn delete_pipeline(id: i32) -> Result<()> {

}

/*
GET    /api/connectors             # List available connector types
GET    /api/transformations        # List available transformation types  
GET    /api/destinations           # List available destination types
*/

async fn get_connectors() -> Vec<String> {

}

async fn get_transformations() -> Vec<String> {

}

async fn get_destinations() -> Vec<String> {

}

/*
POST   /api/pipelines/{id}/connectors      # Add connector to pipeline
PUT    /api/pipelines/{id}/connectors/{cid} # Edit connector
DELETE /api/pipelines/{id}/connectors/{cid} # Remove connector

POST   /api/pipelines/{id}/transformations  # Add transformation
PUT    /api/pipelines/{id}/transformations/{tid} # Edit transformation
DELETE /api/pipelines/{id}/transformations/{tid} # Remove transformation

POST   /api/pipelines/{id}/destinations     # Add destination
PUT    /api/pipelines/{id}/destinations/{did} # Edit destination  
DELETE /api/pipelines/{id}/destinations/{did} # Remove destination
*/



/*
GET    /api/pipelines/{id}/dag     # Get pipeline DAG
PUT    /api/pipelines/{id}/dag     # Update connections/links
*/

/*
GET    /api/pipelines/{id}/state   # Get pipeline state
GET    /api/connectors/{id}/state  # Get connector state
POST   /api/connectors/{id}/console # Enable/disable console logging
*/