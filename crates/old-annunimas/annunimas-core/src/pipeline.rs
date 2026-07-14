// sigil: REPAIR
use crate::error::Result;
use crate::ledger::Ledger;
use crate::message::Message;
use crate::router::Router;
use crate::task::Task;

pub struct Pipeline {
    router: Router,
    ledger: Ledger,
    joule_budget: u64,
}

impl Pipeline {
    pub fn new(router: Router, ledger: Ledger, joule_budget: u64) -> Self {
        Self {
            router,
            ledger,
            joule_budget,
        }
    }

    pub async fn submit(&self, mut task: Task) -> Result<Task> {
        self.ledger.append(&Message::event(
            "ceo",
            "task_received",
            serde_json::to_value(&task)?,
        ))?;
        // JouleWork check
        let estimated_cost = 10;
        if estimated_cost > self.joule_budget {
            task.fail("JouleWork budget exceeded");
            return Ok(task);
        }
        let agent = self.router.route(&task)?;
        let agent_name = agent.name().to_string();
        task.assign(&agent_name);
        self.ledger
            .append(&Message::task_assignment(task.id, &agent_name))?;

        match agent.execute(&mut task).await {
            Ok(()) => {
                self.ledger.append(&Message::task_complete(
                    task.id,
                    &agent_name,
                    task.result.clone().unwrap_or(serde_json::Value::Null),
                ))?;
            }
            Err(e) => {
                let reason = e.to_string();
                task.fail(&reason);
                self.ledger
                    .append(&Message::task_failed(task.id, &agent_name, &reason))?;
            }
        }

        Ok(task)
    }
}
