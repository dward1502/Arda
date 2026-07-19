opentelemetry + opentelemetry-otlp crates (and tracing-opentelemetry for easy integration with tracing).Basic Setup (add to your workspace deps):toml

[workspace.dependencies]
opentelemetry = { version = "0.27", features = ["trace"] }
opentelemetry-otlp = { version = "0.27", features = ["tonic"] }
opentelemetry-semantic-conventions = "0.27"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-opentelemetry = "0.28"

Instrument an Agent Run (example in engine or a service):rust

use opentelemetry::{trace::{Tracer, TraceContextExt}, KeyValue};
use tracing::{info_span, Instrument};
use opentelemetry_semantic_conventions as semconv;

async fn run_agent_task(task: String) {
    let tracer = opentelemetry::global::tracer("arda-agent");

    let span = tracer.start_with_context("invoke_agent", &opentelemetry::Context::current());
    let _guard = tracing::span!(tracing::Level::INFO, "invoke_agent",
        "gen_ai.operation.name" = "invoke_agent",
        "agent.name" = "hermes",
        "gen_ai.conversation.id" = uuid::Uuid::new_v4().to_string()
    ).entered();

    // LLM call
    {
        let _llm_span = info_span!("gen_ai.chat",
            "gen_ai.system" = "openai",
            "gen_ai.request.model" = "gpt-4o",
            "gen_ai.usage.input_tokens" = 420
        ).entered();
        // ... call your LLM
    }

    // Tool / file operation
    {
        let _tool_span = info_span!("execute_tool write_file",
            "gen_ai.tool.name" = "write_file",
            "file.path" = "/Arda/outputs/reports/summary.md"
        ).entered();
        // safe file write via your FileService
    }
}

For File Folder Discipline:Add custom spans/attributes on every file operation (file.path, file.category, file.validated).
In traces you’ll instantly see when an agent tried a bad path and got corrected.

Recommended Backends for Agentic DevGrafana Tempo + Loki 

https://medium.com/@venkat65534/full-stack-observability-with-grafana-prometheus-loki-tempo-and-opentelemetry-90839113d17d 
This website is important for telemtry. i have grafana and prometheus running on beelink we can add loki tempo and opentelemtry and get better data and observation. 