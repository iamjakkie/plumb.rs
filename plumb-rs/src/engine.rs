use std::collections::HashMap;

use futures::future::join_all;
use plumb_src::{
    Connector, CsvConfig, CsvError, CsvSource, FilterConfig, FilterTransform, MapConfig,
    MapTransform, PreviewConfig, RateLimitConfig, RateLimitTransform, Transformation,
    WebSocketConfig, WebSocketSource,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::topo::topo_sort;

pub type Record = Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecNode {
    pub id: i32,
    /// `connector` | `transformation` | `destination`
    pub kind: String,
    pub name: String,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecEdge {
    pub from: i32,
    pub to: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    pub pipeline_id: i32,
    pub nodes: Vec<SpecNode>,
    pub edges: Vec<SpecEdge>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EngineEventKind {
    Record,
    Error,
    Status,
    Eof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineEvent {
    pub pipeline_id: i32,
    pub node_id: Option<i32>,
    pub kind: EngineEventKind,
    pub payload: Value,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid graph: {0}")]
    InvalidGraph(String),
    #[error("invalid config on node {node}: {message}")]
    InvalidConfig { node: i32, message: String },
    #[error("runtime error on node {node}: {message}")]
    Runtime { node: i32, message: String },
    #[error("{0}")]
    Other(String),
}

fn operator_type(config: &Value) -> Option<&str> {
    config
        .get("type")
        .and_then(|v| v.as_str())
        .or_else(|| config.get("operator").and_then(|v| v.as_str()))
}

fn emit(
    tx: &broadcast::Sender<EngineEvent>,
    pipeline_id: i32,
    node_id: Option<i32>,
    kind: EngineEventKind,
    payload: Value,
) {
    let _ = tx.send(EngineEvent {
        pipeline_id,
        node_id,
        kind,
        payload,
    });
}

fn parse_csv_config(config: &Value, node_id: i32) -> Result<CsvConfig, EngineError> {
    let file_path = config
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| EngineError::InvalidConfig {
            node: node_id,
            message: "file_path is required".into(),
        })?
        .to_string();
    let has_headers = config
        .get("has_headers")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let delimiter = config
        .get("delimiter")
        .and_then(|v| v.as_str())
        .and_then(|s| s.chars().next());
    Ok(CsvConfig {
        file_path,
        has_headers,
        delimiter,
    })
}

/// Run a pipeline until completion, cancellation, or fatal error.
pub async fn run(
    spec: PipelineSpec,
    cancel: CancellationToken,
    events: broadcast::Sender<EngineEvent>,
) -> Result<(), EngineError> {
    if spec.nodes.is_empty() {
        return Err(EngineError::InvalidGraph("no nodes".into()));
    }

    let sources: Vec<_> = spec
        .nodes
        .iter()
        .filter(|n| n.kind == "connector")
        .collect();
    if sources.len() != 1 {
        return Err(EngineError::InvalidGraph(
            "MVP requires exactly one connector source".into(),
        ));
    }

    let _order = topo_sort(&spec.nodes, &spec.edges)?;

    let mut indegree: HashMap<i32, usize> = HashMap::new();
    let mut out_edges: HashMap<i32, Vec<i32>> = HashMap::new();
    for e in &spec.edges {
        *indegree.entry(e.to).or_default() += 1;
        out_edges.entry(e.from).or_default().push(e.to);
    }
    for n in &spec.nodes {
        let deg = indegree.get(&n.id).copied().unwrap_or(0);
        match n.kind.as_str() {
            "connector" if deg != 0 => {
                return Err(EngineError::InvalidGraph(format!(
                    "connector '{}' must not have inputs",
                    n.name
                )));
            }
            "transformation" | "destination" if deg != 1 => {
                return Err(EngineError::InvalidGraph(format!(
                    "node '{}' must have exactly one input (got {deg})",
                    n.name
                )));
            }
            _ => {}
        }
    }

    let mut senders: HashMap<(i32, i32), mpsc::Sender<Record>> = HashMap::new();
    let mut receivers: HashMap<(i32, i32), mpsc::Receiver<Record>> = HashMap::new();
    for e in &spec.edges {
        let (tx, rx) = mpsc::channel::<Record>(64);
        senders.insert((e.from, e.to), tx);
        receivers.insert((e.from, e.to), rx);
    }

    let pipeline_id = spec.pipeline_id;

    emit(
        &events,
        pipeline_id,
        None,
        EngineEventKind::Status,
        json!({"status": "running"}),
    );

    // Brief pause so the UI can subscribe to SSE before records start flowing.
    tokio::select! {
        _ = cancel.cancelled() => return Ok(()),
        _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {}
    }

    let mut handles = Vec::new();

    for node in &spec.nodes {
        let node = node.clone();
        let outs: Vec<mpsc::Sender<Record>> = out_edges
            .get(&node.id)
            .into_iter()
            .flatten()
            .filter_map(|to| senders.get(&(node.id, *to)).cloned())
            .collect();

        let mut input_rx: Option<mpsc::Receiver<Record>> = None;
        for e in &spec.edges {
            if e.to == node.id {
                input_rx = receivers.remove(&(e.from, e.to));
                break;
            }
        }

        let cancel = cancel.clone();
        let events = events.clone();
        let kind = node.kind.clone();

        let handle = tokio::spawn(async move {
            let result = match kind.as_str() {
                "connector" => run_source(pipeline_id, node, outs, cancel, events.clone()).await,
                "transformation" => {
                    run_transform(pipeline_id, node, input_rx, outs, cancel, events.clone()).await
                }
                "destination" => {
                    run_destination(pipeline_id, node, input_rx, cancel, events.clone()).await
                }
                other => Err(EngineError::InvalidGraph(format!("unknown kind '{other}'"))),
            };
            if let Err(ref e) = result {
                emit(
                    &events,
                    pipeline_id,
                    None,
                    EngineEventKind::Error,
                    json!({"error": e.to_string()}),
                );
            }
            result
        });
        handles.push(handle);
    }

    drop(senders);

    let results = join_all(handles).await;
    let mut first_err: Option<EngineError> = None;
    for r in results {
        match r {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(EngineError::Other(format!("task join: {e}")));
                }
            }
        }
    }

    if cancel.is_cancelled() {
        emit(
            &events,
            pipeline_id,
            None,
            EngineEventKind::Status,
            json!({"status": "stopped"}),
        );
        return Ok(());
    }

    if let Some(err) = first_err {
        emit(
            &events,
            pipeline_id,
            None,
            EngineEventKind::Status,
            json!({"status": "error", "error": err.to_string()}),
        );
        return Err(err);
    }

    emit(
        &events,
        pipeline_id,
        None,
        EngineEventKind::Eof,
        json!({"status": "completed"}),
    );
    emit(
        &events,
        pipeline_id,
        None,
        EngineEventKind::Status,
        json!({"status": "idle"}),
    );

    Ok(())
}

async fn forward_to_all(outs: &[mpsc::Sender<Record>], record: Record) -> bool {
    for tx in outs {
        if tx.send(record.clone()).await.is_err() {
            return false;
        }
    }
    true
}

async fn run_source(
    pipeline_id: i32,
    node: SpecNode,
    outs: Vec<mpsc::Sender<Record>>,
    cancel: CancellationToken,
    events: broadcast::Sender<EngineEvent>,
) -> Result<(), EngineError> {
    let op = operator_type(&node.config).ok_or_else(|| EngineError::InvalidConfig {
        node: node.id,
        message: "missing config.type".into(),
    })?;

    match op {
        "csv" => {
            let cfg = parse_csv_config(&node.config, node.id)?;
            let mut source = CsvSource::connect(cfg).await.map_err(|e| EngineError::Runtime {
                node: node.id,
                message: e.to_string(),
            })?;

            loop {
                if cancel.is_cancelled() {
                    let _ = source.close().await;
                    break;
                }
                match source.next().await {
                    Some(Ok(bytes)) => {
                        let record: Value = serde_json::from_slice(&bytes).map_err(|e| {
                            EngineError::Runtime {
                                node: node.id,
                                message: e.to_string(),
                            }
                        })?;
                        if !forward_to_all(&outs, record).await {
                            break;
                        }
                    }
                    Some(Err(CsvError::EndOfFile)) => break,
                    Some(Err(e)) => {
                        return Err(EngineError::Runtime {
                            node: node.id,
                            message: e.to_string(),
                        });
                    }
                    None => break,
                }
            }
            let _ = source.close().await;
        }
        "websocket" => {
            let url = node
                .config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| EngineError::InvalidConfig {
                    node: node.id,
                    message: "url is required".into(),
                })?
                .to_string();
            let subscription_message = node
                .config
                .get("subscription_message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let cfg = WebSocketConfig {
                url,
                subscription_message,
                headers: None,
            };
            let mut source =
                WebSocketSource::connect(cfg)
                    .await
                    .map_err(|e| EngineError::Runtime {
                        node: node.id,
                        message: e.to_string(),
                    })?;

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = source.close().await;
                        break;
                    }
                    item = source.next() => {
                        match item {
                            Some(Ok(bytes)) => {
                                let record: Value = match serde_json::from_slice(&bytes) {
                                    Ok(v) => v,
                                    Err(_) => json!({"raw": String::from_utf8_lossy(&bytes)}),
                                };
                                if !forward_to_all(&outs, record).await {
                                    break;
                                }
                            }
                            Some(Err(e)) => {
                                emit(
                                    &events,
                                    pipeline_id,
                                    Some(node.id),
                                    EngineEventKind::Error,
                                    json!({"error": e.to_string()}),
                                );
                                return Err(EngineError::Runtime {
                                    node: node.id,
                                    message: e.to_string(),
                                });
                            }
                            None => break,
                        }
                    }
                }
            }
            let _ = source.close().await;
        }
        other => {
            return Err(EngineError::InvalidConfig {
                node: node.id,
                message: format!("unknown connector type '{other}'"),
            });
        }
    }
    Ok(())
}

async fn run_transform(
    _pipeline_id: i32,
    node: SpecNode,
    input: Option<mpsc::Receiver<Record>>,
    outs: Vec<mpsc::Sender<Record>>,
    cancel: CancellationToken,
    _events: broadcast::Sender<EngineEvent>,
) -> Result<(), EngineError> {
    let mut input = input.ok_or_else(|| {
        EngineError::InvalidGraph(format!("transform '{}' has no input", node.name))
    })?;

    let op = operator_type(&node.config).ok_or_else(|| EngineError::InvalidConfig {
        node: node.id,
        message: "missing config.type".into(),
    })?;

    let node_id = node.id;
    match op {
        "filter" => {
            let field = node
                .config
                .get("field")
                .and_then(|v| v.as_str())
                .ok_or_else(|| EngineError::InvalidConfig {
                    node: node_id,
                    message: "field is required".into(),
                })?
                .to_string();
            let cfg = FilterConfig {
                field,
                op: node
                    .config
                    .get("op")
                    .and_then(|v| v.as_str())
                    .unwrap_or("eq")
                    .to_string(),
                value: node
                    .config
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };
            let mut t = FilterTransform::new(cfg)
                .await
                .map_err(|e| EngineError::InvalidConfig {
                    node: node_id,
                    message: e.to_string(),
                })?;
            pump_transform(node_id, &mut t, &mut input, &outs, &cancel).await
        }
        "map" => {
            let cfg = MapConfig {
                fields: node
                    .config
                    .get("fields")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                rename: node
                    .config
                    .get("rename")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };
            let mut t = MapTransform::new(cfg)
                .await
                .map_err(|e| EngineError::InvalidConfig {
                    node: node_id,
                    message: e.to_string(),
                })?;
            pump_transform(node_id, &mut t, &mut input, &outs, &cancel).await
        }
        "rate_limit" => {
            let cfg = RateLimitConfig {
                records_per_second: node
                    .config
                    .get("records_per_second")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(5.0),
            };
            let mut t = RateLimitTransform::new(cfg)
                .await
                .map_err(|e| EngineError::InvalidConfig {
                    node: node_id,
                    message: e.to_string(),
                })?;
            pump_transform(node_id, &mut t, &mut input, &outs, &cancel).await
        }
        other => Err(EngineError::InvalidConfig {
            node: node_id,
            message: format!("unknown transform type '{other}'"),
        }),
    }
}

async fn pump_transform<T>(
    node_id: i32,
    transform: &mut T,
    input: &mut mpsc::Receiver<Record>,
    outs: &[mpsc::Sender<Record>],
    cancel: &CancellationToken,
) -> Result<(), EngineError>
where
    T: Transformation,
    T::Error: std::fmt::Display,
{
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            item = input.recv() => {
                match item {
                    Some(record) => {
                        let out = transform.apply(record).await.map_err(|e| {
                            EngineError::Runtime {
                                node: node_id,
                                message: e.to_string(),
                            }
                        })?;
                        if let Some(record) = out {
                            if !forward_to_all(outs, record).await {
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
        }
    }
    Ok(())
}

async fn run_destination(
    pipeline_id: i32,
    node: SpecNode,
    input: Option<mpsc::Receiver<Record>>,
    cancel: CancellationToken,
    events: broadcast::Sender<EngineEvent>,
) -> Result<(), EngineError> {
    let mut input = input.ok_or_else(|| {
        EngineError::InvalidGraph(format!("destination '{}' has no input", node.name))
    })?;

    let op = operator_type(&node.config).unwrap_or("preview");
    if op != "preview" {
        return Err(EngineError::InvalidConfig {
            node: node.id,
            message: format!("unknown destination type '{op}'"),
        });
    }

    let sample_every = node
        .config
        .get("sample_every")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1);
    let _cfg = PreviewConfig {
        sample_every,
    };
    let mut count: u64 = 0;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            item = input.recv() => {
                match item {
                    Some(record) => {
                        count += 1;
                        if count % sample_every == 0 {
                            emit(
                                &events,
                                pipeline_id,
                                Some(node.id),
                                EngineEventKind::Record,
                                record,
                            );
                        }
                    }
                    None => break,
                }
            }
        }
    }
    Ok(())
}
