use anyhow::Result;
use axum::{
    extract::{Multipart, Path, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{cors::{Any, CorsLayer}, services::ServeDir};

mod db;
mod models;

use db::Database;
use models::{Node, Pipeline, ValidationResult};

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
        .route("/api/connectors", get(list_connectors))
        .route("/api/pipelines", get(list_pipelines).post(create_pipeline))
        .route("/api/pipelines/{id}", get(get_pipeline).delete(delete_pipeline))
        .route("/api/pipelines/{id}/nodes", post(create_node))
        .route("/api/pipelines/{id}/edges", post(create_edge))
        .route("/api/pipelines/{id}/validate", get(validate_pipeline))
        .route("/api/pipelines/{id}/trigger", post(trigger_pipeline))
        .route("/api/upload", post(upload_file))
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

async fn list_connectors() -> Json<Vec<serde_json::Value>> {
    let connectors = plumb_src::get_available_connectors()
        .into_iter()
        .map(|(type_id, config_schema, constraint_schema, display_name, description)| {
            serde_json::json!({
                "type": type_id,
                "display_name": display_name,
                "description": description,
                "config_schema": config_schema,
                "constraint_schema": constraint_schema,
            })
        })
        .collect();
    Json(connectors)
}

async fn list_pipelines(State(db): State<Arc<Database>>) -> ApiResult<Json<Vec<Pipeline>>> {
    Ok(Json(db.get_all_pipelines()?))
}

async fn get_pipeline(
    State(db): State<Arc<Database>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Pipeline>> {
    Ok(Json(db.get_pipeline(id)?))
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

#[derive(Deserialize)]
struct EdgePayload {
    from: i32,
    to: i32,
}

async fn create_edge(
    State(db): State<Arc<Database>>,
    Path(pipeline_id): Path<i32>,
    Json(edge): Json<EdgePayload>,
) -> ApiResult<StatusCode> {
    db.add_edge(pipeline_id, edge.from, edge.to)?;
    Ok(StatusCode::CREATED)
}

#[derive(Serialize)]
struct UploadResponse {
    path: String,
    filename: String,
    size: usize,
}

async fn upload_file(mut multipart: Multipart) -> ApiResult<Json<UploadResponse>> {
    let uploads_dir = PathBuf::from("./data/uploads");
    fs::create_dir_all(&uploads_dir)?;

    while let Some(field) = multipart.next_field().await? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload".to_string());

        // Sanitise: strip directory components, keep only the filename
        let filename = PathBuf::from(&filename)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "upload".to_string());

        let data = field.bytes().await?;
        let size = data.len();
        let dest = uploads_dir.join(&filename);
        fs::write(&dest, &data)?;

        let abs = fs::canonicalize(&dest)?;
        return Ok(Json(UploadResponse {
            path: abs.to_string_lossy().into_owned(),
            filename,
            size,
        }));
    }

    Err(anyhow::anyhow!("No file in request").into())
}

async fn validate_pipeline(
    State(db): State<Arc<Database>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<ValidationResult>> {
    Ok(Json(db.validate_pipeline(id)?))
}

async fn trigger_pipeline(
    State(db): State<Arc<Database>>,
    Path(id): Path<i32>,
) -> ApiResult<Json<ValidationResult>> {
    let result = db.validate_pipeline(id)?;
    if result.valid {
        db.update_pipeline_status(id, models::PipelineStatus::Running)?;
    }
    Ok(Json(result))
}
