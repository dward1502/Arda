# arda-aule ownership

Crate: `crates/spine/observability/arda-aule`
Owner: ARDA / observability layer
Status: active
Reviewed: 2026-07-28

## This crate owns

- Stable observability contracts, readiness summaries, and governance-metric rendering.
- Feature-gated telemetry event and tracing/export configuration surfaces.
- CEO/autonomy-profile, Prometheus projection, council-gate, heartbeat, planning, registry, and service-observability implementations under `full-cli`.
- Governed autopilot coordination and durable queue/intent production, but not execution authority.
- The `arda-cli` operator binary assembled from these supported surfaces.
- Prometheus exposition, including bounded Vaire/Mnemosyne observability consumption under `http`.

## This crate does not own

- Provider/model selection, route fitness, fleet routing, or inference policy; Manwe owns them.
- Governance decisions or policy authority; `arda-governance` supplies those contracts.
- Source memory, recall, consolidation, or promotion truth; Vaire/Mnemosyne owns it.
- Task execution, daemon supervision, deployment credentials, or operator state.
- Prometheus deployment, scrape configuration, dashboards, or alert-rule rollout.

## Change authority

- Default-surface changes require no-default checks/tests and downstream contract review.
- Telemetry API changes require Manwe `telemetry` compilation and test construction.
- `full-cli` changes require all-feature process tests, strict Clippy, and strict Rustdoc.
- Metrics ABI changes require coordinated scrape/dashboard/alert consumer migration; do not rename existing `annunimas_*` series in isolation.
- Vaire metric-input changes require exporter process tests against the exact producer schema.
- Do not restore archived plans or retired surfaces without a concrete owner, live consumer, and tested contract.
