// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { ArdaBundle } from '../../../lib/ardaSource'
import { createArdaFleetViewModel } from './ardaAdapter'

function bundleWith(overrides: Partial<ArdaBundle>): ArdaBundle {
  return overrides as ArdaBundle
}

describe('ardaAdapter', () => {
  it('maps ARDA fleet projections into the universal Fleet view model', () => {
    const bundle = bundleWith({
      generatedAt: '2026-06-26T00:00:00Z',
      operatorRuntimeStatus: {
        summary: {
          fleet_live_llm_nodes_total: 2,
          fleet_routable_local_providers_total: 1,
          unexpected_offline_total: 1,
        },
        fleet: { targets_total: 3 },
        lane_routes: {
          interactive: {
            provider_id: 'ollama-local',
            model_id: 'qwen2.5-coder',
            route_class: 'local',
            reason: 'lowest latency',
          },
        },
        lane_headroom: {
          interactive: { 'ollama-local': 4 },
          execution: { 'ollama-local': 2 },
          background: { 'ollama-local': 8 },
        },
        lane_fitness: {
          interactive: {
            'ollama-local': {
              avg_latency_ms: 95,
              success_count: 9,
              failure_count: 1,
            },
          },
        },
        routable_providers: [
          {
            provider_id: 'ollama-local',
            models: [{ id: 'qwen2.5-coder', context_window: 32768, healthy: true, is_default: true }],
            avg_latency_ms: 95,
            active_connections: 1,
          },
        ],
      },
    })

    const model = createArdaFleetViewModel(bundle)

    expect(model.roleId).toBe('fleet')
    expect(model.status).toBe('attention')
    expect(model.metrics).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: 'total_targets', value: 3 }),
      expect.objectContaining({ id: 'live_targets', value: 2 }),
      expect.objectContaining({ id: 'routable_providers', value: 1 }),
      expect.objectContaining({ id: 'unexpected_offline', value: 1 }),
    ]))
    expect(model.providers[0]).toEqual(expect.objectContaining({
      providerId: 'ollama-local',
      providerName: 'ollama-local',
      healthy: true,
      avgLatencyMs: 95,
    }))
    expect(model.laneOwnership[0]).toEqual(expect.objectContaining({ lane: 'interactive' }))
    expect(model.sources.map((source) => source.id)).toContain('operator_runtime_status')
    expect('raw' in model).toBe(false)
  })

  it('returns a safe fallback instead of raw JSON when ARDA projections are missing', () => {
    const model = createArdaFleetViewModel(bundleWith({ generatedAt: '2026-06-26T00:00:00Z' }))

    expect(model.status).toBe('empty')
    expect(model.providers).toEqual([])
    expect(model.sources.every((source) => source.freshness.status === 'missing')).toBe(true)
    expect(model.summary[0]).toMatch(/projection unavailable/i)
    expect('raw' in model).toBe(false)
  })

  it('joins the six fleet projections into one topology model with honest source states', () => {
    const bundle = bundleWith({
      generatedAt: '2026-08-16T12:00:00Z',
      operatorRuntimeStatus: { summary: {}, fleet: {} },
      fleetRuntime: { generated_at_utc: '2026-08-16T11:59:00Z' },
      fleetNodes: {
        generated_at_utc: '2026-08-16T11:59:00Z',
        counts: { configured_total: 2, online_total: 1 },
        nodes: [
          { display_name: 'Core', configured: { id: 'core', hostname: 'core', node_class: 'core_compute' }, observed: { online: true } },
          { display_name: 'Backbone', configured: { id: 'backbone', hostname: 'backbone', node_class: 'backbone_compute' }, observed: { online: false } },
        ],
      },
      fleetModels: { generated_at_utc: '2026-08-16T11:59:00Z', configured_nodes: [] },
      fleetHealth: { generated_at_utc: '2026-08-16T11:59:00Z', cleanup_summary: { status: 'recent_offline_observed' } },
      fleetHardware: { generated_at_utc: '2026-08-16T11:59:00Z', configured_nodes: [] },
      fleetBackbone: { generated_at_utc: '2026-08-16T11:59:00Z', backbone_node: { id: 'backbone' } },
    })

    const model = createArdaFleetViewModel(bundle)

    expect(model.nodes).toEqual([
      expect.objectContaining({ id: 'core', online: true, nodeClass: 'core_compute' }),
      expect.objectContaining({ id: 'backbone', online: false, nodeClass: 'backbone_compute' }),
    ])
    expect(model.backboneNodeId).toBe('backbone')
    expect(model.sources.map((source) => source.id)).toEqual(expect.arrayContaining([
      'fleet_runtime', 'fleet_nodes', 'fleet_models', 'fleet_health', 'fleet_hardware', 'fleet_backbone',
    ]))
    expect(model.sources.filter((source) => source.id.startsWith('fleet_')).every((source) => source.freshness.status === 'fresh')).toBe(true)
  })
})
