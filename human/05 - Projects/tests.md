---
title: "Tests"
last_updated: 2026-05-14
soterion:
  type: project_summary
  category: summaries
  project: annunimas
  agent_access: public
  mnemosyne_priority: high
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# tests/

**Purpose**: Integration tests for Annunimas system

## Test Files (11 total)

| File | Coverage |
|------|----------|
| `test_pipeline.rs` | Full pipeline flow |
| `test_governance.rs` | Triad, Game Theory, Resonance |
| `test_soterion.rs` | Soterion index & YAML parsing |
| `test_soterion_watcher.rs` | Soterion file watcher |
| `test_soterion_auto_inject_and_scan.rs` | Auto-injection |
| `test_love_resonance.rs` | Love/Resonance scoring |
| `test_graph_build.rs` | Knowledge graph building |
| `test_warden.rs` | Warden monitoring |
| `test_alert_logging.rs` | Alert system |
| `test_agent_soterion.rs` | Agent Soterion integration |
| `test_registry.rs` | Agent registry |

## Key Test Coverage

### Pipeline Tests
- Full pipeline: Task → Router → AthenaAgent → Complete
- Verifies resonance calculation
- Verifies game theory scoring

### Governance Tests
- Triad validation: valid vs invalid tasks
- Game theory scoring: reputation, win_rate
- Love resonance: time harmony, phi harmonic

### Soterion Tests
- Index creation and querying
- YAML frontmatter parsing
- Sigil and realm lookups
- High resonance finding

---

## Thoughts

**Good:**
- Good coverage of core systems
- Tests governance (triad, resonance, game theory)
- Tests Soterion indexing and parsing
- Tests pipeline end-to-end

**Needs Work:**
- No unit tests visible (only integration)
- Some tests may have issues (test_governance.rs line 24: Ledger::new("temp") - string not Path)
- Missing: agent-specific tests (Hermes, Oracle, Plutus)
- Missing: CLI tests
- Missing: stress/load tests

**Improve:**
- Add more unit tests
- Fix any broken tests
- Add agent-specific tests
- Add CLI tests
- Add integration with actual LLM providers
- Add performance tests
