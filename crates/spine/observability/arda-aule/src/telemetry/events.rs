// sigil: TELEMETRY
use crate::telemetry::{Destination, TelemetryEvent};

pub(crate) trait TelemetryEmitter {
    fn emit(&self, event: &TelemetryEvent);
}

#[derive(Debug, Default)]
pub(crate) struct CompositeEmitter {
    pub(crate) tracers: Vec<Box<dyn TelemetryEmitter + Send + Sync>>,
}

impl TelemetryEmitter for CompositeEmitter {
    fn emit(&self, event: &TelemetryEvent) {
        for emitter in &self.tracers {
            emitter.emit(event);
        }
    }
}

pub(crate) fn emit(event: TelemetryEvent) {
    CompositeEmitter::default().emit(&event);
}

pub(crate) fn emit_agent_command(crate_name: &str, command: &str, status: &str) {
    emit(
        TelemetryEvent::new("agent.command")
            .destination(Destination::Both)
            .attr("crate", crate_name)
            .attr("command", command)
            .attr("status", status),
    );
}

pub(crate) fn emit_governance_triad(crate_name: &str, triage: &str, gate_state: &str) {
    emit(
        TelemetryEvent::new("governance.triad")
            .destination(Destination::Both)
            .attr("crate", crate_name)
            .attr("triage", triage)
            .attr("gate_state", gate_state),
    );
}
