arda-aule Cleanup & Improvement Plan
====================================

Status: complete.

Completed scope
---------------

1. Attached CEO core-link, pipeline, and router modules under `full-cli`.
2. Attached coherent Prometheus projections, council, heartbeat, orders, planner, registry,
   service, thought, IPC, and optional HTTP modules.
3. Attached CEO autopilot and replaced its removed Apollo dependency with a truthful handoff to
   the canonical task queue; Aule leaves work pending for the active core loop/executor authority.
4. Assigned provider/fleet routing to Manwe. Aule now emits execution intents with
   `routing_authority: "manwe"` instead of importing retired `annunimas_fleet` types.
5. Added Prometheus status, thought, escalation, roster, planning, drift, council, execution-intent,
   runtime-reconciliation, daemon, and autopilot commands to the one supported `arda-cli` binary.
6. Retired duplicate crate roots, the unused package stub, the stale fleet pipeline, the replaced
   Apollo bridge, the detached autopilot binary, and the copied global legacy CLI tree.
7. Re-baselined default, `full-cli`, all-feature, formatting, and strict-Clippy checks.

Closeout
--------

No Aule-owned consolidation tasks remain. Commands from the detached global CLI copy that belong to
other crates were intentionally not re-homed in Aule; their canonical owners remain responsible for
their operator surfaces.