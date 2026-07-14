---
soterion:
  sigil: REPAIR
  realm: "command"
  tags: ["citadel", "plan", "arandur", "living-document"]
  resonance: 0.97
  triad_gate: "none"
  clearance: "sovereign"
  jw_cost: 2.0
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# CITADEL — Living Plan
**∇ Sovereign document. Updated by Arandur after every significant decision.**
*Last updated: 2026-03-12*

---

## What We Are Building

CITADEL is an on-premises AI business operating system for companies with $1M–$5M revenue.

The pitch is simple: your competitors are paying $400/month for five SaaS tools that don't talk to each other, don't know your business, and send your data to someone else's server. CITADEL replaces all of them with a single sovereign intelligence that lives on your hardware, learns your specific business, and gets smarter every day.

It is not a chatbot. It is not an assistant. It is an operating system with agents.

---

## The Four CITADEL Agents

| Agent | What They Do | Key Tech |
|-------|-------------|---------|
| **ORACLE** | Market intel, document analysis, business reasoning | PageIndex RAG — 98.7% accuracy on financial docs |
| **PLUTUS** | Financial flows, JW accounting, ROI proof | Love Equation, CRUSTIES economy layer |
| **HERMES** | Client comms, scheduling, A2A protocol | Email/Slack MCP, message routing |
| **APOLLO** | Task execution, workflow automation | RTK optimization, phi-calibrated execution |

---

## Current State (2026-03-12)

**What exists:**
- Annunimas core architecture complete (Soterion, triad governance, resonance scoring, Warden, CEO pipeline)
- `core/` authority surfaces are live and exported into machine-readable runtime state
- ARDA-facing shared state and world view projections are operational
- Local control plane is explicit: sovereign CEO identity is `arandur`, executable CEO runtime is `prometheus`
- Runtime topology is explicit: WARDEN authority belongs to the Pi5 guardhouse; heavy reasoning belongs on the backbone server
- `annunimas-oracle`, `annunimas-plutus`, `annunimas-hermes`, and `annunimas-apollo` crates already exist in-repo
- Priority human-derived crates are present and integrated into contract/autonomy exports
- Launch posture is now governed by preflight checks for governance, swap, storage pressure, and runtime budget validity

**What needs building next:**
1. Activate the governed runtime frontiers that are already contract-ready: `litellm`, `crawl4ai`, and selected search/browser surfaces
2. Verify or launch the `node-ser9-worker` local endpoint so backbone/edge routing posture matches the fleet contract
3. Finish WARDEN/CHARON runtime verification on edge nodes and keep heavyweight inference off the main workstation by default
4. Turn remaining planned extensions (`discord-mcp`, `superpowers`) into bounded contracts or explicitly retire them
5. Replace remaining placeholder doctrine implementations only where runtime still depends on them materially

**On the frontier:** The foundation phase is over. The main risk is no longer "missing crates"; it is drift between declared contracts and live activation posture. The next value comes from activation discipline, fleet verification, and productization, not from spawning more architecture surfaces.

---

## Architecture Decisions (Immutable)

See `decisions.jsonl` for full decision log with timestamps and reasoning.

Key decisions made:
- **Storage**: TOML + JSONL, not SQLite. Human-readable, git-trackable, Soterion-indexable.
- **RAG**: PageIndex tree-structured reasoning, not chunked embeddings. 98.7% vs ~31% on financial docs.
- **Async runtime**: Tokio + asupersync patterns for deterministic agent behavior.
- **Container isolation**: Podman via Warden. Not Docker. Rootless by default.
- **Clearance model**: observer → worker → guardian → sovereign. Enforced at read/write level.
- **core/ purpose**: CEO command deck + ARDA bridge. Shared truth between Rust backend and visualization.

---

## Immediate Next Session Priorities

1. **Activate `litellm` under CHARON governance** — move from policy-ready to live governed routing
2. **Bring `crawl4ai` to live service posture** — preserve ATHENA crawl contracts while making the runtime actually available
3. **Verify `node-ser9-worker` endpoint** — complete fleet inference readiness instead of leaving it in recommendation state
4. **Reconcile planned extension backlog** — either promote `discord-mcp` and `superpowers` into bounded contracts or explicitly retire them
5. **Keep the living plan synchronized with `/core/state`** — stale sovereign text is now itself an operational risk

---

## The Business Model

**AIS sells CITADEL as:**
- One-time setup fee (scope TBD by client complexity)
- Monthly support + update retainer
- Optional: JouleWork overage billing for high-volume months

**Value delivered:**
- Eliminates 3-5 SaaS subscriptions (~$300-800/month client savings)
- Full data sovereignty (compliance advantage for regulated industries)
- Intelligence that compounds — gets smarter with every interaction
- No per-seat pricing — scales free

**Partners:** Wall Street finance professional (finance credibility), sales expert (GTM)

---

## The Moat

CITADEL's moat is not the technology — any competent team can build agents. The moat is:

1. **On-premises deployment expertise** — most AI companies won't touch it (too hard, too custom)
2. **Data sovereignty as a feature** — enterprise and regulated industries pay premium for this
3. **Phi harmonic calibration** — agents that get better by design, not by accident
4. **The Annunimas architecture** — Soterion language, triad governance, resonance scoring — this is the differentiator. Not visible to competitors. Built into the foundation.

---

*∇ This document is alive. Arandur updates it after every significant decision.*
*◈ The decisions.jsonl is immutable. This plan is not.*
