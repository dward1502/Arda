---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
sigil: "∇"
domain: "fleet"
purpose: "Load-based fleet routing with governance integration"
status: "active"
references:
  - "core/state/fleet_capability_ranking.json"
  - "core/state/multi_domain_routing_contract.json"
  - "config/fleet.toml"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# arda-fleet

Load-based fleet routing with triad governance integration.

## Purpose

Distribute agent spawning across edge devices when local capacity is exceeded, using the governance system (triad, joulework, love equation) to validate routing decisions.

## Integration with Core Contracts

- **Fleet Capability Ranking**: Uses `core/state/fleet_capability_ranking.json` for execution_authority (primary/specialized/secondary)
- **Multi-Domain Routing**: Follows doctrine from `core/state/multi_domain_routing_contract.json`
- **OpenCode Route Governor**: Writes decisions through sovereign route contract

## Governance Flow

```
Task → Triad Gate (Aurelius/Bacon/Sun Tzu) → JouleWork Profile → Love Equation Score → Fleet Decision
```

## Usage

```rust
use arda_fleet::FleetCapacityManager;

let manager = FleetCapacityManager::new("/var/home/arda")?;
let decision = manager.evaluate_task(&task);

if decision.decision.is_accepted() {
    // Dispatch to edge node
}
```
