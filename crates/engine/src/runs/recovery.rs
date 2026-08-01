use super::{RunEvent, RunEventKind, RunStoreError};
use arda_core::run_graph::RunGraph;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RecoveredRun {
    pub events: Vec<RunEvent>,
    pub checkpoint: Option<RunGraph>,
    pub applied_idempotency_keys: HashMap<String, u64>,
}

impl RecoveredRun {
    pub(crate) fn from_parts(events: Vec<RunEvent>, checkpoint: Option<RunGraph>) -> Self {
        let applied_idempotency_keys = events
            .iter()
            .map(|event| (event.idempotency_key.clone(), event.sequence))
            .collect();
        Self {
            events,
            checkpoint,
            applied_idempotency_keys,
        }
    }

    pub fn replay(&self, base: &RunGraph) -> Result<RunGraph, RunStoreError> {
        base.validate().map_err(RunStoreError::Graph)?;
        let mut projection = base.clone();
        for event in &self.events {
            if let RunEventKind::NodeTransition { state } = &event.kind {
                projection
                    .transition_node(&event.node_id, *state)
                    .map_err(RunStoreError::Graph)?;
            }
        }
        projection.validate().map_err(RunStoreError::Graph)?;
        Ok(projection)
    }
}
