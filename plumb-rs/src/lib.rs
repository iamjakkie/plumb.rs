//! In-process stream engine for plumb local MVP.

mod engine;
mod topo;

pub use engine::{run, EngineError, EngineEvent, EngineEventKind, PipelineSpec, SpecEdge, SpecNode};
pub use topo::topo_sort;

pub type Record = serde_json::Value;
