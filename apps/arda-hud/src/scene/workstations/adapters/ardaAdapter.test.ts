// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { ArdaBundle } from '../../../lib/ardaSource'
import { createArdaContinuityViewModel, createArdaFleetHealth, createArdaFleetViewModel, createArdaRoutingViewModel } from './ardaAdapter'

function bundleWith(overrides: Partial<ArdaBundle>): ArdaBundle {
  return overrides as ArdaBundle
}

describe('ardaAdapter', () => {
  it('preserves distinct fleet operational failures from the fleet health authority', () => {
    const health = createArdaFleetHealth(bundleWith({
      fleetHealth: {
        operational_states: {
          ready_total: 2,
          intentional_offline_total: 1,
          unobserved_total: 3,
          unreachable_total: 4,
          service_down_total: 5,
          routing_drift_total: 6,
        },
      },
      operatorRuntimeStatus: {
        summary: { fleet_routable_local_providers_total: 2 },
      },
    }))

    expect(health).toEqual(expect.objectContaining({
      liveTargets: 13,
      intentionalOffline: 1,
      unexpectedOffline: 7,
      unobserved: 3,
      unreachable: 4,
      serviceDown: 5,
      routingDrift: 6,
      attentionTotal: 18,
    }))
  })

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

  it('derives routing ownership, pressure, fitness, communication pathways, and source truth without Fleet health', () => {
    const bundle = bundleWith({
      generatedAt: '2026-08-16T12:00:00Z',
      operatorRuntimeStatus: {
        generated_at_utc: '2026-08-16T11:59:00Z',
        charon: {
          providers_healthy: 2,
          providers_enabled: 3,
          recent_route_successes: 9,
          recent_route_failures: 1,
          budget_pressure: { highest_level: 'warning', cooldown_total: 1 },
        },
        lane_routes: { interactive: { provider_id: 'groq', model_id: 'fast', route_class: 'cloud', reason: 'low latency' } },
        lane_headroom: { interactive: { groq: 4 } },
        lane_fitness: { interactive: { groq: { avg_latency_ms: 80, success_count: 9, failure_count: 1 } } },
        routable_providers: [{ provider_id: 'groq', active_connections: 2, soft_caps: { interactive: 6 } }],
      },
      providerIntelligence: {
        generated_at_utc: '2026-08-16T11:58:00Z',
        providers: {
          groq: { enabled: true, healthy: true, models: [{ id: 'fast' }] },
          cerebras: { enabled: true, healthy: true, models: [{ id: 'reasoning' }], access_tier: 'free_cloud' },
        },
      },
      hermesMessages: [{ platform: 'discord', status: 'delivered' }],
      hermesAgentGatewayReceipts: [{ platform: 'telegram', status: 'accepted' }],
    })

    const model = createArdaRoutingViewModel(bundle)

    expect(model.roleId).toBe('routing')
    expect(model.lanes[0]).toEqual(expect.objectContaining({ lane: 'interactive', providerId: 'groq', headroom: 4 }))
    expect(model.providers[0]).toEqual(expect.objectContaining({ providerId: 'groq', activeConnections: 2 }))
    expect(model.providers).toContainEqual(expect.objectContaining({ providerId: 'cerebras', activeConnections: 0, modelCount: 1 }))
    expect(model.routeHistory).toEqual({ successes: 9, failures: 1 })
    expect(model.budgetPressure).toEqual(expect.objectContaining({ highestLevel: 'warning', cooldownTotal: 1 }))
    expect(model.communicationPathways.map((pathway) => pathway.id)).toEqual(['discord', 'telegram'])
    expect(model.sources.map((source) => source.id)).toEqual(expect.arrayContaining(['operator_runtime_status', 'provider_intelligence', 'manwe_router', 'chronos_runtime']))
    expect(model.metrics.map((metric) => metric.id)).not.toContain('unexpected_offline')
  })

  it('derives distinct human, business, and personal horizons with missing-reference and value truth', () => {
    const bundle = bundleWith({
      generatedAt: '2026-08-16T12:00:00Z',
      humanContext: {
        generated_at_utc: '2026-08-16T11:59:00Z',
        human_portal: { docs: [{ title: 'Context', path: 'human/context.md', body_preview: 'Current context' }], notes: [] },
      },
      businessRuntime: {
        generated_at_utc: '2026-08-16T11:58:00Z',
        client_records: [
          { path: 'data/business/clients/live.json', exists: true, body_preview: 'Active engagement' },
          { path: 'data/business/clients/gone.json', exists: false, body_preview: 'Old engagement' },
        ],
        state: { offers: [{ id: 'offer', title: 'Advisory offer', status: 'internal-first', description: 'Planned product' }] },
        company_ops: {
          opportunities: [{ opportunity_id: 'opp', title: 'Future work', forecast_value: { amount_minor: 50000, currency: 'USD' } }],
          engagements: [{ engagement_id: 'paid', title: 'Delivered work', state: 'paid', realized_value: { amount_minor: 25000, currency: 'USD', outcome_receipt_id: 'receipt-1' } }],
          projects: [{ project_id: 'gone-project', title: 'Gone project', path: 'data/projects/gone/project.json', exists: false }],
        },
      },
      personalRuntime: {
        generated_at_utc: '2026-08-16T11:57:00Z',
        highlights: { priorities: ['Family continuity'], values: ['truth'] },
        documents: [{ title: 'Life audit', path: 'data/personal/audit.md', body_preview: 'In progress' }],
      },
    })

    const model = createArdaContinuityViewModel(bundle)

    expect(model.roleId).toBe('continuity')
    expect(model.horizons.map((horizon) => horizon.id)).toEqual(['human', 'business', 'personal'])
    expect(model.items).toContainEqual(expect.objectContaining({ horizon: 'business', path: 'data/business/clients/gone.json', state: 'missing' }))
    expect(model.valueTruth).toEqual({ plannedMinor: 50000, realizedMinor: 25000, currency: 'USD', realizedReceiptCount: 1 })
    expect(model.items).toEqual(expect.arrayContaining([
      expect.objectContaining({ title: 'Future work', state: 'planned' }),
      expect.objectContaining({ title: 'Delivered work', state: 'realized' }),
      expect.objectContaining({ title: 'Gone project', state: 'missing' }),
    ]))
    expect(model.items.find((item) => item.title === 'Family continuity')).toEqual(expect.objectContaining({ horizon: 'personal' }))
  })
})
