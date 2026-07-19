# Manwe / provider catalog plan

Source of truth for fleet nodes: `config/fleet.toml`

## Confirmed live nodes

| node id | role | endpoint | health/models | models served | status |
|---|---|---|---|---|---|
| node-core-hub | main_hub | `100.78.138.113:9337` | HTTP 200 | LFM2.5-8B-A1B-Q4_K_M | active |
| node-pi5-warden | warden_guardhouse | `100.110.85.37:1234` | HTTP 200 | Qwen3.5-4B-Q4_K_M.gguf | active |
| node-ser9-worker | ser9_sovereign_worker | `100.103.125.88:9337` | HTTP 200 | Qwen_Qwen3.5-4B-Q6_K | active |
| node-backbone-server | backbone_fast_general | `100.102.250.115:8093` | HTTP 200 | lfm2.5-8b-a1b-q4km | active |
| node-backbone-gemma4-coder | backbone_coder | `100.102.250.115:8094` | HTTP 200 | gemma4-12b-coder-q4km | active |
| node-backbone-vision | backbone_vision | `100.102.250.115:8081` | HTTP 200 | Qwen2.5-VL-7B-Instruct | active |

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

## Recommended next steps

- Implement `ProviderCatalog::from_fleet_config()` in `manwe`.
- Update `manwe` bootstrap to load `config/fleet.toml` before falling back to
  `default_bootstrap()`.
- Wire successful catalog load into `manwe`’s `/v1/models` and `/health`
  endpoints so HUD/Charon see live fleet state.
- Re-evaluate `node-ser9-carnice` once a replacement llama.cpp service exists;
  do not add stale entries to the catalog.

## Fleet -> provider mapping draft

- node-core-hub -> `edge_core`
- node-pi5-warden -> `edge_guardhouse`
- node-ser9-worker -> `edge_beelink_light`
- node-backbone-server -> `edge_backbone`
- node-backbone-gemma4-coder -> `edge_backbone_coder`
- node-backbone-vision -> `edge_backbone_vision`
- local fallback -> `local_placeholder`
