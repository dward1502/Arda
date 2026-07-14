// sigil: REPAIR
use crate::message::{A2AMessage, DeliveryStatus};
use crate::registry::AgentRegistry;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MessageRouter {
    queues: std::collections::HashMap<String, VecDeque<QueuedMessage>>,
    dead_letter: VecDeque<QueuedMessage>,
    max_queue_size: usize,
}

#[derive(Clone)]
pub struct QueuedMessage {
    pub message: A2AMessage,
    pub enqueued_at: chrono::DateTime<chrono::Utc>,
    pub attempts: u32,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            queues: std::collections::HashMap::new(),
            dead_letter: VecDeque::new(),
            max_queue_size: 1000,
        }
    }

    pub fn with_max_queue_size(mut self, size: usize) -> Self {
        self.max_queue_size = size;
        self
    }

    pub fn route(&mut self, msg: &A2AMessage, registry: &AgentRegistry) -> RouteResult {
        let target = msg.to.clone();

        let agent = registry.get(&target);

        match agent {
            Some(a) if a.is_available() => RouteResult::Deliver(a.endpoint.clone()),
            Some(a) => {
                self.enqueue(target.clone(), msg.clone());
                RouteResult::Queued(a.id.clone())
            }
            None => RouteResult::UnknownAgent(target),
        }
    }

    pub fn enqueue(&mut self, agent_id: String, message: A2AMessage) {
        let queue = self.queues.entry(agent_id).or_default();

        if queue.len() >= self.max_queue_size {
            self.dead_letter.push_back(QueuedMessage {
                message,
                enqueued_at: chrono::Utc::now(),
                attempts: 0,
            });
        } else {
            queue.push_back(QueuedMessage {
                message,
                enqueued_at: chrono::Utc::now(),
                attempts: 0,
            });
        }
    }

    pub fn dequeue(&mut self, agent_id: &str) -> Option<A2AMessage> {
        self.queues
            .get_mut(agent_id)
            .and_then(|q| q.pop_front())
            .map(|qm| qm.message)
    }

    pub fn peek(&self, agent_id: &str) -> Option<&A2AMessage> {
        self.queues
            .get(agent_id)
            .and_then(|q| q.front())
            .map(|qm| &qm.message)
    }

    pub fn queue_len(&self, agent_id: &str) -> usize {
        self.queues.get(agent_id).map(|q| q.len()).unwrap_or(0)
    }

    pub fn total_queued(&self) -> usize {
        self.queues.values().map(|q| q.len()).sum()
    }

    pub fn retry_failed(&mut self) -> Vec<A2AMessage> {
        let mut retried = Vec::new();

        for queue in self.queues.values_mut() {
            for qm in queue.iter_mut() {
                if qm.message.delivery_status == DeliveryStatus::Failed {
                    qm.attempts += 1;
                    if qm.attempts < 3 {
                        qm.message.delivery_status = DeliveryStatus::Pending;
                        retried.push(qm.message.clone());
                    }
                }
            }
        }

        retried
    }

    pub fn drain_expired(&mut self) -> usize {
        let mut drained = 0;

        for queue in self.queues.values_mut() {
            queue.retain(|qm| {
                let keep = !qm.message.is_expired();
                if !keep {
                    drained += 1;
                }
                keep
            });
        }

        drained
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteResult {
    Deliver(Option<String>),
    Queued(String),
    UnknownAgent(String),
    Broadcast(Vec<String>),
}

impl RouteResult {
    pub fn is_deliverable(&self) -> bool {
        matches!(self, RouteResult::Deliver(Some(_)))
    }

    pub fn target_agent(&self) -> Option<String> {
        match self {
            RouteResult::Deliver(_) => None,
            RouteResult::Queued(a) => Some(a.clone()),
            RouteResult::UnknownAgent(a) => Some(a.clone()),
            RouteResult::Broadcast(agents) => agents.first().cloned(),
        }
    }
}

pub type SharedRouter = Arc<RwLock<MessageRouter>>;

impl MessageRouter {
    pub fn shared() -> SharedRouter {
        Arc::new(RwLock::new(Self::new()))
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}
