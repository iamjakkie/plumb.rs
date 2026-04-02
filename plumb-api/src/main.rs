use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use std::{env, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{cors::{Any, CorsLayer}, services::ServeDir};

mod db;
mod models;

use db::Database;
use models::*;

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}

type ApiResult<T> = Result<T, AppError>;

// ---------------------------------------------------------------------------
// Server bootstrap
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let db = Arc::new(Database::new()?);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    let api = Router::new()
        .route("/health", get(health))
        .route("/api/pipelines", get(list_pipelines).post(create_pipeline))
        .route(
            "/api/pipelines/{id}",
            get(get_pipeline).delete(delete_pipeline),
        )
        .route("/api/pipelines/{id}/nodes", post(create_node))
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(db);

    let app = api.fallback_service(ServeDir::new("plumb-ui"));

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    println!("Listening on http://{}:{}", host, port);
    axum::serve(listener, app).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> &'static str {
    "OK"
}

async fn list_pipelines(
    State(db): State<Arc<Database>>,
) -> ApiResult<Json<Vec<Pipeline>>> {
    let pipelines = db.get_all_pipelines()?;
    Ok(Json(pipelines))
}

async fn get_pipeline(
    State(db): State<Arc<Database>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Pipeline>> {
    let pipeline = db.get_pipeline(id)?;
    Ok(Json(pipeline))
}

async fn create_pipeline(
    State(db): State<Arc<Database>>,
    Json(payload): Json<Pipeline>,
) -> ApiResult<Json<Pipeline>> {
    let pipeline = if payload.nodes.is_empty() && payload.edges.is_empty() {
        let new = Pipeline::new(payload.name);
        let id = db.add_pipeline(&new)?;
        Pipeline { id, ..new }
    } else {
        db.clone_pipeline(&payload)?
    };

    Ok(Json(pipeline))
}

async fn delete_pipeline(
    State(db): State<Arc<Database>>,
    Path(id): Path<i32>,
) -> ApiResult<StatusCode> {
    db.remove_pipeline(id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_node(
    State(db): State<Arc<Database>>,
    Path(pipeline_id): Path<i32>,
    Json(node): Json<Node>,
) -> ApiResult<Json<Node>> {
    let id = db.add_node(pipeline_id, &node)?;
    Ok(Json(Node { id, ..node }))
}
