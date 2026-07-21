# Manwe / provider catalog plan

Source of truth for fleet nodes: `config/fleet.toml`

## Confirmed live nodes

|| node id | role | endpoint | health/models | models served | status |
| --- | --- | --- | --- | --- | --- |
|| node-core-hub | main_hub | `annunimas-core:9337` | HTTP 200 | LFM2.5-8B-A1B-Q4_K_M | active |
|| node-pi5-warden | warden_guardhouse | `warden:1234` | HTTP 200 | Qwen3.5-4B-Q4_K_M.gguf | active |
|| node-ser9-worker | ser9_sovereign_worker | `beelink:9337` | HTTP 200 | Ternary-Bonsai-8B-Q2_0 | active |
|| node-backbone-server | backbone_fast_general | `annunimas-server:8093` | HTTP 200 | lfm2.5-8b-a1b-q4km | inactive |
|| node-backbone-gemma4-coder | backbone_coder | `annunimas-server:8094` | HTTP 200 | qwen2.5-coder-7b-q4km | inactive |
|| node-backbone-vision | backbone_vision | `annunimas-server:8081` | HTTP 200 | Qwen2.5-VL-7B-Instruct | inactive |
|| node-backbone-bonsai27 | backbone_ternary_27b | `annunimas-server:8095` | HTTP 200 | ternary-bonsai-27b-q2_0 | active |

## Offline / inactive nodes

| node id | reason |
|---|---|
| node-ser9-carnice | `:1234` not listening per fleet notes; old distrobox service removed |
| node-laptop | enrolled as optional voice/ASR, not inference |
| node-pi5-citadel-avatar | `:8080` unreachable from current probe; avatar service, not LLM lane |

## What this means for `manwe`

`manwe/src/provider.rs` currently only provides a static bootstrap catalog with
a `local_placeholder` entry. It does not read `config/fleet.toml`, so manwe
cannot yet route to the live nodes above.

## Proposed provider.rs behavior

1. Keep `ProviderCatalog::default_bootstrap()` as a safety net when
   `config/fleet.toml` is missing or malformed.
2. Add a `ProviderCatalog::from_fleet_config(path)` that:
   - reads `config/fleet.toml`,
   - filters to nodes whose `health_url`/`models_url` respond,
   - maps each live node to a `ProviderDefinition`,
   - preserves `charon_provider_id` as the provider id so HUD/routing stays
     aligned with Charon routing docs.
3. Add a `ProviderCatalog::refresh()` path or `manwe` startup probe so
   runtime health is re-evaluated without restart.
4. Preserve the existing local HTTP transport shape; fleet nodes are
   OpenAI-compatible or llama.cpp, both covered by `ProviderTransport::OpenAICompatible`.

## Implementation status

- Implemented `ProviderCatalog::from_fleet_config()` and `from_fleet_config_direct()` in `manwe`.
- Implemented `ProviderDefinition::from_fleet_node(...)` mapping.
- Implemented `ProviderCatalog::refresh()` for live re-evaluation without restart.
- Updated `ProviderCatalog::default_bootstrap()` to load `config/fleet.toml` first, then fall back to `local_placeholder`.
- Wired `default_bootstrap()` into manwe startup in `src/main.rs`: `AppState` now carries the fleet catalog, `/v1/models` prefers fleet providers when present, and `/v1/capabilities` reports `fleet_providers`.
- `/health` is exposed on the adaptive service surface as an HTTP route; the stable gateway HTTP surface uses `/v1/capabilities` + `/healthz` for health/model visibility.

## Recommended next steps

- Re-evaluate `node-ser9-carnice` once a replacement llama.cpp service exists; do not add stale entries to the catalog. (I dont think we need this running if we have bonzai 27B running on beelink instead only need 1 model to run services on the beelink correct?)

## Fleet -> provider mapping draft

- node-core-hub -> `edge_core`
- node-pi5-warden -> `edge_guardhouse`
- node-ser9-worker -> `edge_beelink_light`
- node-backbone-server -> `edge_backbone`
- node-backbone-gemma4-coder -> `edge_backbone_coder`
- node-backbone-vision -> `edge_backbone_vision`
- node-backbone-bonsai27 -> `edge_backbone_bonsai27`
- local fallback -> `local_placeholder`
