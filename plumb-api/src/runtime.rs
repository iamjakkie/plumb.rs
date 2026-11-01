use std::collections::HashMap;
use std::sync::Arc;

use plumb_rs::{run, EngineEvent, PipelineSpec, SpecEdge, SpecNode};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::db::Database;
use crate::models::{Pipeline, PipelineStatus};

struct RunningPipeline {
    cancel: CancellationToken,
    handle: JoinHandle<()>,
    events: broadcast::Sender<EngineEvent>,
}

pub struct RuntimeManager {
    running: Mutex<HashMap<i32, RunningPipeline>>,
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(HashMap::new()),
        }
    }

    pub async fn is_running(&self, pipeline_id: i32) -> bool {
        self.running.lock().await.contains_key(&pipeline_id)
    }

    pub async fn subscribe(&self, pipeline_id: i32) -> Option<broadcast::Receiver<EngineEvent>> {
        self.running
            .lock()
            .await
            .get(&pipeline_id)
            .map(|r| r.events.subscribe())
    }

    pub async fn start(
        self: &Arc<Self>,
        db: Arc<Database>,
        pipeline: Pipeline,
    ) -> Result<broadcast::Receiver<EngineEvent>, String> {
        let id = pipeline.id;
        {
            let guard = self.running.lock().await;
            if guard.contains_key(&id) {
                return Err("pipeline is already running".into());
            }
        }

        let spec = pipeline_to_spec(&pipeline);
        let (events_tx, events_rx) = broadcast::channel::<EngineEvent>(1024);
        let cancel = CancellationToken::new();
        let cancel_child = cancel.clone();
        let events_for_task = events_tx.clone();
        let manager = Arc::clone(self);
        let db_task = Arc::clone(&db);

        let handle = tokio::spawn(async move {
            let result = run(spec, cancel_child, events_for_task).await;
            let status = match &result {
                Ok(()) => PipelineStatus::Idle,
                Err(_) => PipelineStatus::Error,
            };
            let _ = db_task.update_pipeline_status(id, status);
            manager.running.lock().await.remove(&id);
        });

        self.running.lock().await.insert(
            id,
            RunningPipeline {
                cancel,
                handle,
                events: events_tx,
            },
        );

        Ok(events_rx)
    }

    pub async fn stop(&self, pipeline_id: i32) -> Result<(), String> {
        let entry = {
            let mut guard = self.running.lock().await;
            guard.remove(&pipeline_id)
        };
        let Some(entry) = entry else {
            return Err("pipeline is not running".into());
        };
        entry.cancel.cancel();
        let _ = entry.handle.await;
        Ok(())
    }
}

fn pipeline_to_spec(pipeline: &Pipeline) -> PipelineSpec {
    PipelineSpec {
        pipeline_id: pipeline.id,
        nodes: pipeline
            .nodes
            .iter()
            .map(|n| SpecNode {
                id: n.id,
                kind: n.node_type.as_str().to_string(),
                name: n.name.clone(),
                config: n.config.clone(),
            })
            .collect(),
        edges: pipeline
            .edges
            .iter()
            .map(|e| SpecEdge {
                from: e.from,
                to: e.to,
            })
            .collect(),
    }
}
