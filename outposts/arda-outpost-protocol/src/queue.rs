use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::{OutpostObservation, SCHEMA_VERSION};

#[derive(Debug, thiserror::Error)]
pub enum OutpostQueueError {
    #[error("queue is closed")]
    Closed,
    #[error("observation schema mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: String, actual: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckStatus {
    Acknowledged,
    Failed,
    Pending,
}

#[derive(Debug, Clone)]
pub struct QueuedObservation {
    pub observation: OutpostObservation,
    pub attempts: usize,
    pub last_error: Option<String>,
    pub ack: AckStatus,
}

#[derive(thiserror::Error, Debug)]
pub enum ProduceError {
    #[error("queue full")]
    Full,
    #[error("schema mismatch: expected `{0}`, got `{1}`")]
    SchemaMismatch(String, String),
    #[error("queue closed")]
    Closed,
}

#[derive(thiserror::Error, Debug)]
#[error("queue not found for topic `{0}`")]
pub struct SubscribeError(pub String);

#[derive(thiserror::Error, Debug)]
#[error("observe error: sender closed")]
pub struct ObserveError;

impl From<SubscribeError> for ProduceError {
    fn from(_error: SubscribeError) -> Self {
        OutpostQueueError::Closed.into()
    }
}

impl From<OutpostQueueError> for ProduceError {
    fn from(error: OutpostQueueError) -> Self {
        match error {
            OutpostQueueError::Closed => ProduceError::Closed,
            OutpostQueueError::SchemaMismatch { expected, actual } => {
                ProduceError::SchemaMismatch(expected, actual)
            }
        }
    }
}

#[derive(Debug)]
struct InnerQueue {
    topic: String,
    max_capacity: usize,
    schema_version: String,
    buffer: VecDeque<QueuedObservation>,
    in_flight: VecDeque<QueuedObservation>,
}

#[derive(Debug, Clone)]
pub struct OutpostQueue {
    topics: Arc<Mutex<Vec<InnerQueue>>>,
    max_capacity: usize,
}

impl OutpostQueue {
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            topics: Arc::new(Mutex::new(Vec::new())),
            max_capacity,
        }
    }

    pub fn create_topic(&self, topic: impl Into<String>) -> Result<(), SubscribeError> {
        let topic = topic.into();
        let mut guard = self.topics.lock();
        if guard.iter().any(|item| item.topic == topic) {
            return Ok(());
        }
        let max = self.max_capacity;
        guard.push(InnerQueue {
            topic,
            max_capacity: max,
            schema_version: SCHEMA_VERSION.to_string(),
            buffer: VecDeque::with_capacity(max),
            in_flight: VecDeque::with_capacity(max),
        });
        Ok(())
    }

    pub fn produce(
        &self,
        topic: impl Into<String>,
        observation: &OutpostObservation,
    ) -> Result<QueuedObservation, ProduceError> {
        let topic = topic.into();
        let mut guard = self.topics.lock();
        let inner = guard
            .iter_mut()
            .find(|item| item.topic == topic)
            .ok_or_else(|| SubscribeError(topic.clone()))?;

        if observation.schema_version != inner.schema_version {
            return Err(ProduceError::SchemaMismatch(
                inner.schema_version.clone(),
                observation.schema_version.clone(),
            ));
        }

        if inner.buffer.len() + inner.in_flight.len() >= inner.max_capacity {
            return Err(ProduceError::Full);
        }

        let queued = QueuedObservation {
            observation: observation.clone(),
            attempts: 1,
            last_error: None,
            ack: AckStatus::Pending,
        };
        inner.buffer.push_back(queued.clone());
        Ok(queued)
    }

    pub fn consume(
        &self,
        topic: impl Into<String>,
    ) -> Result<Option<QueuedObservation>, ObserveError> {
        let topic = topic.into();
        let mut guard = self.topics.lock();
        let inner = guard
            .iter_mut()
            .find(|item| item.topic == topic)
            .ok_or(ObserveError)?;

        let queued = inner.buffer.pop_front();
        if let Some(queued) = &queued {
            inner.in_flight.push_back(queued.clone());
        }
        Ok(queued)
    }

    pub fn ack(
        &self,
        topic: impl Into<String>,
        queued: &QueuedObservation,
        status: AckStatus,
    ) -> Result<(), ObserveError> {
        let topic = topic.into();
        let mut guard = self.topics.lock();
        let inner = guard
            .iter_mut()
            .find(|item| item.topic == topic)
            .ok_or(ObserveError)?;
        let position = inner
            .in_flight
            .iter()
            .position(|item| item.observation.id == queued.observation.id)
            .ok_or(ObserveError)?;

        match status {
            AckStatus::Acknowledged => {
                inner.in_flight.remove(position);
            }
            AckStatus::Failed => {
                let mut retry = inner.in_flight.remove(position).ok_or(ObserveError)?;
                retry.attempts += 1;
                retry.last_error = Some("consumer reported failure".to_string());
                retry.ack = AckStatus::Pending;
                inner.buffer.push_back(retry);
            }
            AckStatus::Pending => {
                inner.in_flight[position].ack = AckStatus::Pending;
            }
        }
        Ok(())
    }

    pub fn close(&self, topic: impl Into<String>) -> Result<(), SubscribeError> {
        let topic = topic.into();
        let mut guard = self.topics.lock();
        if let Some(pos) = guard.iter().position(|item| item.topic == topic) {
            guard.remove(pos);
        }
        Ok(())
    }
}

pub fn generate_queue(max_capacity: usize) -> OutpostQueue {
    OutpostQueue::with_capacity(max_capacity)
}

pub fn consume_queue(
    queue: &OutpostQueue,
    topic: impl Into<String>,
) -> Result<Option<QueuedObservation>, ObserveError> {
    queue.consume(topic)
}
