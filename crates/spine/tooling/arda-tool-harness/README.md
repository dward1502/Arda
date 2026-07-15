---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# arda-tool-harness

Spawned from the Arda sovereign crate blueprint. This crate is currently a
blueprint contract for governed tool invocation, not the active mutating-tool
runtime boundary.

- Realm: `operations`
- Productizable: `true`
- Role: blueprint contract
- Required exports: `core/state/arda-tool-harness.json`
- Required hooks: task ledger, ARDA visibility, Soterion trace, governance validators, memory checkpoint capture

## Baseline

This crate blueprint starts with:

- crate contract in `src/contract.rs`
- tool metadata, invocation-envelope, and route-plan primitives in `src/types.rs`
- service status in `src/service.rs`
- governance smoke test in `tests/contract_smoke.rs`

Any new agentic crate should preserve this baseline rather than retrofitting governance, memory, and state-export posture later.

## Usage

```rust
use arda_tool_harness::service::build_invocation_plan;
use arda_tool_harness::types::{
    InvocationDisposition, InvocationEnvelope, RiskLevel, SideEffectClass, ToolMetadata,
};

let metadata = ToolMetadata {
    tool_id: "tool.demo".into(),
    version: "1".into(),
    owner: "athena".into(),
    description: "demo".into(),
    input_schema_ref: "schema/in.json".into(),
    output_schema_ref: "schema/out.json".into(),
    risk_level: RiskLevel::High,
    side_effect_class: SideEffectClass::Mutating,
};
let envelope = InvocationEnvelope {
    trace_id: Some("trace-1".into()),
    actor: Some("athena".into()),
    idempotency_key: Some("idem-1".into()),
};

let plan = build_invocation_plan(&metadata, &envelope).expect("valid invocation");
assert_eq!(plan.disposition, InvocationDisposition::AllowMutatingWithIdempotency);
```

## Extension Points

- Add concrete tool adapters only after an owning runtime path exists.
- Keep mutating tools behind trace, actor, idempotency, and governance checks.
- Preserve behavioral tests for denied tools, replay/idempotency, and trace continuity.
