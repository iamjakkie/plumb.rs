use anyhow::Result;
use axum::{Json, Router, extract::State, routing::get};
use dotenv::dotenv;
use std::{env, sync::Arc};

mod db;
mod models;

use db::Database;
use models::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    println!("Starting Plumb API server on {}:{}", host, port);

    let db = Arc::new(Database::new()?);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/pipelines", get(list_pipelines).post(create_pipeline))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    println!("Server listening on http://{}:{}", host, port);
    axum::serve(listener, app).await?;

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

async fn list_pipelines(State(db): State<Arc<Database>>) -> Result<Json<Vec<Pipeline>>, String> {
    let pipelines = db
        .get_all_pipelines()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(Json(pipelines))
}

async fn create_pipeline(
    State(db): State<Arc<Database>>,
    Json(pipeline): Json<Pipeline>,
) -> Result<Json<Pipeline>, String> {
    let created_pipeline = if pipeline.nodes.is_empty() && pipeline.edges.is_empty() {
        let id = db.add_pipeline(&pipeline).map_err(
            |e| format!("Database error: {}", e)
        )?;
        Pipeline {
            id,
            name: pipeline.name,
            nodes: vec![],
            edges: vec![]
        }
    } else {
        db.clone_pipeline(&pipeline).map_err(|e| format!("Could not clone pipeline: {}", e))?
    };

    Ok(Json(created_pipeline))
}

// async fn get_pipeline(id: i32) -> Result<Pipeline> {

// }

// async fn update_pipeline(pipeline: Pipeline) -> Result<()> {

// }

// async fn delete_pipeline(id: i32) -> Result<()> {

// }

// /*
// GET    /api/connectors             # List available connector types
// GET    /api/transformations        # List available transformation types
// GET    /api/destinations           # List available destination types
// */
// async fn get_connectors() -> Vec<String> {

// }

// async fn get_transformations() -> Vec<String> {

// }

// async fn get_destinations() -> Vec<String> {

// }

// /*
// POST   /api/pipelines/{id}/connectors      # Add connector to pipeline
// PUT    /api/pipelines/{id}/connectors/{cid} # Edit connector
// DELETE /api/pipelines/{id}/connectors/{cid} # Remove connector

// POST   /api/pipelines/{id}/transformations  # Add transformation
// PUT    /api/pipelines/{id}/transformations/{tid} # Edit transformation
// DELETE /api/pipelines/{id}/transformations/{tid} # Remove transformation

// POST   /api/pipelines/{id}/destinations     # Add destination
// PUT    /api/pipelines/{id}/destinations/{did} # Edit destination
// DELETE /api/pipelines/{id}/destinations/{did} # Remove destination
// */
// async fn add_node(connector: Node) -> Result<()> {

// }

// async fn edit_node(id: i32, connector: Node) -> Result<()> {

// }

// async fn delete_node(id: i32) -> Result<()> {

// }

// /*
// GET    /api/pipelines/{id}/dag     # Get pipeline DAG
// PUT    /api/pipelines/{id}/dag     # Update connections/links
// */
// /*
// GET    /api/pipelines/{id}/state   # Get pipeline state
// GET    /api/connectors/{id}/state  # Get connector state
// POST   /api/connectors/{id}/console # Enable/disable console logging
// */
