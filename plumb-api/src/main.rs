use anyhow::Result;
use axum::{
    extract::{Multipart, Path, State},
    http::{Method, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, env, fs, path::PathBuf, sync::Arc, time::Duration};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

mod db;
mod models;
mod runtime;

use db::Database;
use models::{Node, Pipeline, PipelineStatus, ValidationResult};
use runtime::RuntimeManager;

// ---------------------------------------------------------------------------
// App state + errors
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
    runtime: Arc<RuntimeManager>,
}

struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn internal(err: impl ToString) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.message });
        (self.status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        let msg = e.to_string();
        if msg.contains("not found") {
            Self::new(StatusCode::NOT_FOUND, msg)
        } else {
            Self::internal(msg)
        }
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

    let state = AppState {
        db: Arc::new(Database::new()?),
        runtime: Arc::new(RuntimeManager::new()),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    let api = Router::new()
        .route("/health", get(health))
        .route("/api/connectors", get(list_connectors))
        .route("/api/operators", get(list_operators))
        .route("/api/pipelines", get(list_pipelines).post(create_pipeline))
        .route(
            "/api/pipelines/{id}",
            get(get_pipeline).delete(delete_pipeline),
        )
        .route("/api/pipelines/{id}/nodes", post(create_node))
        .route("/api/pipelines/{id}/edges", post(create_edge))
        .route("/api/pipelines/{id}/validate", get(validate_pipeline))
        .route("/api/pipelines/{id}/trigger", post(trigger_pipeline))
        .route("/api/pipelines/{id}/stop", post(stop_pipeline))
        .route("/api/pipelines/{id}/events", get(pipeline_events))
        .route("/api/upload", post(upload_file))
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(state);

    let app = api.fallback_service(ServeDir::new("plumb-ui"));

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    println!("Listening on http://{host}:{port}");
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
        .map(
            |(type_id, config_schema, constraint_schema, display_name, description)| {
                serde_json::json!({
                    "type": type_id,
                    "display_name": display_name,
                    "description": description,
                    "config_schema": config_schema,
                    "constraint_schema": constraint_schema,
                })
            },
        )
        .collect();
    Json(connectors)
}

async fn list_operators() -> Json<Vec<serde_json::Value>> {
    let operators = plumb_src::get_available_operators()
        .into_iter()
        .map(|(kind, type_id, config_schema, display_name, description)| {
            serde_json::json!({
                "kind": kind,
                "type": type_id,
                "display_name": display_name,
                "description": description,
                "config_schema": config_schema,
            })
        })
        .collect();
    Json(operators)
}

async fn list_pipelines(State(state): State<AppState>) -> ApiResult<Json<Vec<Pipeline>>> {
    Ok(Json(state.db.get_all_pipelines()?))
}

async fn get_pipeline(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Pipeline>> {
    Ok(Json(state.db.get_pipeline(id)?))
}

async fn create_pipeline(
    State(state): State<AppState>,
    Json(payload): Json<Pipeline>,
) -> ApiResult<Json<Pipeline>> {
    let pipeline = if payload.nodes.is_empty() && payload.edges.is_empty() {
        let new = Pipeline::new(payload.name);
        let id = state.db.add_pipeline(&new)?;
        Pipeline { id, ..new }
    } else {
        state.db.clone_pipeline(&payload)?
    };
    Ok(Json(pipeline))
}

async fn delete_pipeline(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<StatusCode> {
    if state.runtime.is_running(id).await {
        let _ = state.runtime.stop(id).await;
    }
    state.db.remove_pipeline(id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_node(
    State(state): State<AppState>,
    Path(pipeline_id): Path<i32>,
    Json(node): Json<Node>,
) -> ApiResult<Json<Node>> {
    let _ = state.db.get_pipeline(pipeline_id)?;
    let id = state.db.add_node(pipeline_id, &node)?;
    Ok(Json(Node {
        id,
        pipeline_id,
        ..node
    }))
}

#[derive(Deserialize)]
struct EdgePayload {
    from: i32,
    to: i32,
}

async fn create_edge(
    State(state): State<AppState>,
    Path(pipeline_id): Path<i32>,
    Json(edge): Json<EdgePayload>,
) -> ApiResult<StatusCode> {
    let _ = state.db.get_pipeline(pipeline_id)?;
    state.db.add_edge(pipeline_id, edge.from, edge.to)?;
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
    fs::create_dir_all(&uploads_dir).map_err(AppError::internal)?;

    while let Some(field) = multipart.next_field().await.map_err(AppError::internal)? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload".to_string());

        let filename = PathBuf::from(&filename)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "upload".to_string());

        let data = field.bytes().await.map_err(AppError::internal)?;
        let size = data.len();
        let dest = uploads_dir.join(&filename);
        fs::write(&dest, &data).map_err(AppError::internal)?;

        let abs = fs::canonicalize(&dest).map_err(AppError::internal)?;
        return Ok(Json(UploadResponse {
            path: abs.to_string_lossy().into_owned(),
            filename,
            size,
        }));
    }

    Err(AppError::new(StatusCode::BAD_REQUEST, "No file in request"))
}

async fn validate_pipeline(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<Json<ValidationResult>> {
    Ok(Json(state.db.validate_pipeline(id)?))
}

#[derive(Serialize)]
struct TriggerResponse {
    valid: bool,
    errors: Vec<String>,
    status: String,
}

async fn trigger_pipeline(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<Json<TriggerResponse>> {
    let result = state.db.validate_pipeline(id)?;
    if !result.valid {
        return Ok(Json(TriggerResponse {
            valid: false,
            errors: result.errors,
            status: "idle".into(),
        }));
    }

    if state.runtime.is_running(id).await {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "pipeline is already running",
        ));
    }

    let pipeline = state.db.get_pipeline(id)?;
    state
        .runtime
        .start(Arc::clone(&state.db), pipeline)
        .await
        .map_err(|e| AppError::new(StatusCode::CONFLICT, e))?;

    state
        .db
        .update_pipeline_status(id, PipelineStatus::Running)?;

    Ok(Json(TriggerResponse {
        valid: true,
        errors: vec![],
        status: "running".into(),
    }))
}

async fn stop_pipeline(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<Json<serde_json::Value>> {
    state
        .runtime
        .stop(id)
        .await
        .map_err(|e| AppError::new(StatusCode::CONFLICT, e))?;
    state.db.update_pipeline_status(id, PipelineStatus::Idle)?;
    Ok(Json(serde_json::json!({ "status": "idle" })))
}

async fn pipeline_events(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let rx = state.runtime.subscribe(id).await.ok_or_else(|| {
        AppError::new(
            StatusCode::NOT_FOUND,
            "pipeline is not running (start it first, then connect to events)",
        )
    })?;

    let stream = BroadcastStream::new(rx).filter_map(|item| match item {
        Ok(ev) => {
            let data = serde_json::to_string(&ev).ok()?;
            Some(Ok(Event::default().event("plumb").data(data)))
        }
        Err(_) => None, // lag — drop
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}
