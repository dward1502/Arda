---
soterion:
  sigil: "SCROLL"
  glyph: "📂"
  code_point: "U+1F4C2"
  role: "config_assessment"
  owner: "HADES"
  status: "draft"
  last_reviewed: "2026-07-17"
---

# Config Assessment & Cleanup Plan

Root: `/var/home/mythos/Eregion/Arda/config`

## Current state

`config/` is a flat grab bag: live runtime configs, secret-bearing envs, generated overrides, example/template variants, governance/business configs, Charon/provider/routing definitions, monitoring/systemd artifacts, a stray Rust source file, and an app-specific JSON that belongs under `apps/arda-hud/`.

Stale index drift present: `INDEX.md` lists `llm.toml`/`llm_model_routes.json`; current tree does not contain them. `INDEX.jsonl` is incomplete metadata.

## Classification

### Keep — core runtime/operations
- `default.toml`
- `fleet.toml`
- `charon.providers.toml`
- `runtime_governor_budget.toml`

### Keep but relocate
- `business.toml`, `ceo_startup.yaml`, `matrix_boardrooms.toml`, `federated_comms.toml`, `operator_profile.json`, `goals_seed.json` → `business/`
- `autonomy_operating_loop.toml`, `governance_gates.yaml` → `governance/`
- `selinux_runtime_contract.yaml` → `runtime/` or `systemd/`
- `local_voice_model_lanes.toml`, `model_route_matrix.toml`, `opencode_agent_routes.toml` → `routing/`
- `charon.provider_candidates.toml` → `routing/candidates/`
- `model_registry.toml` → `routing/registry/`
- `hermes_agent_bridge.toml` → `integrations/hermes/`
- `litellm.proxy.yaml`, `llm_usage_limits.yaml` → `integrations/litellm/`
- `chronos_audit_tasks.json` → `integrations/chronos/`
- `arda_hud.settings.json` → `apps/arda-hud/`
- `monitoring-setup/` → `monitoring/`

### Retire / archive
- `annunimas.example.toml`, `annunimas.template.toml`, `annunimas.env.example`
- `.env.generated`, `runtime.generated.env`
- `.env.example`, `runtime.env.example` if env templates move to app/package-local locations
- `offsite_operator.env` → secrets store or gitignore
- `hermes_agent_bridge.example.toml`, `hermes_agent_gateway_annunimas.example.yaml`
- `INDEX.jsonl`
- `gen_keys.rs` → `scripts/` or `tools/`

### Secrets — move only after consumer updates
- `.env`

## Proposed structure

```
config/
├── runtime/
├── routing/
│   └── candidates/
├── governance/
├── business/
├── integrations/
│   ├── hermes/
│   ├── litellm/
│   └── chronos/
├── monitoring/
├── systemd/
├── env/
└── archive/
```

## Assessment

1. Provider/routing configs are duplicated and drifting: `charon.providers.toml` is source of truth; `model_registry.toml`, `model_route_matrix.toml`, `opencode_agent_routes.toml`, and `charon.provider_candidates.toml` should be treated as derived views/intake, not independent truth.
2. Env handling is unclear: multiple templates/generated/local overrides coexist. Canonicalize to one template + one local override; generated files should not be committed.
3. INDEX drift: regenerate after cleanup.
4. App-specific config leaked into repo root: `arda_hud.settings.json` belongs under `apps/arda-hud/`.
5. Legacy Annunimas naming creates confusion: archive/rename old `annunimas.*` configs.
6. Generated artifacts are in tree: `.env.generated`, `runtime.generated.env` should be gitignored or removed.

## Cleanup priority

1. Immediate: retire generated artifacts and stale index metadata
2. Short-term: relocate env/routing/governance/business into domains
3. Medium-term: update consumers to new paths; retire legacy Annunimas copies
4. Ongoing: treat `model_registry.toml`/`model_route_matrix.toml` as derived views, not independent config
