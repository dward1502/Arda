// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  createGenericFleetViewModel,
  createGenericWorkViewModel,
  type GenericControlSystemManifest,
} from './genericControlSystemAdapter'

const fixture: GenericControlSystemManifest = {
  schema_version: 'arda.generic_control_system.v1',
  system_id: 'greenhouse-control',
  title: 'Greenhouse Control',
  generated_at: '2026-06-26T00:00:00Z',
  systems: [
    { id: 'irrigation', label: 'Irrigation', provider_id: 'edge-controller', health: 'healthy' },
    { id: 'climate', label: 'Climate', provider_id: 'edge-controller', health: 'degraded' },
  ],
  services: [
    { id: 'telemetry-api', label: 'Telemetry API', provider_id: 'cloud-ai', health: 'healthy' },
  ],
  providers: [
    {
      id: 'edge-controller',
      name: 'Edge Controller',
      access_tier: 'local',
      quality_band: 'fast',
      enabled: true,
      healthy: true,
      avg_latency_ms: 18,
      active_connections: 2,
      lane_headroom: { interactive: 3, execution: 1, background: 5 },
      models: [
        { id: 'tiny-ops', context_window: 8192, healthy: true, default: true, capable_tasks: ['control', 'triage'] },
      ],
    },
    {
      id: 'cloud-ai',
      name: 'Cloud AI',
      access_tier: 'remote',
      quality_band: 'deep',
      enabled: true,
      healthy: true,
      avg_latency_ms: 220,
      active_connections: 1,
      models: [
        { id: 'planner-large', context_window: 128000, healthy: true, capable_tasks: ['planning'] },
      ],
    },
  ],
  health_checks: [
    { id: 'climate.warning', label: 'Climate warning', status: 'degraded', checked_at: '2026-06-26T00:00:00Z' },
  ],
  tasks: [
    { id: 'task.irrigate.zone-a', title: 'Irrigate zone A', status: 'running', owner: 'edge-controller' },
    { id: 'task.inspect.climate', title: 'Inspect climate drift', status: 'blocked', owner: 'operator' },
  ],
  jobs: [
    { id: 'job.daily-report', title: 'Daily report', status: 'queued', owner: 'cloud-ai' },
  ],
  decisions: [
    { id: 'decision.override-temp', title: 'Override temperature guard', status: 'pending' },
  ],
  source_provenance: [
    { id: 'greenhouse_manifest', label: 'Greenhouse Manifest', path: 'fixtures/greenhouse.json', freshness: 'fresh', timestamp: '2026-06-26T00:00:00Z' },
  ],
  action_descriptors: [
    { id: 'refresh_greenhouse', label: 'Refresh greenhouse state', safety: 'read_only', description: 'Reload the generic manifest.' },
    { id: 'bad_action', label: 'Bad action', safety: 'unsafe' as never },
  ],
  lane_routes: [
    { lane: 'interactive', priority: 'Operator Chat', provider_id: 'edge-controller', model_id: 'tiny-ops', route_class: 'local', reason: 'low latency' },
    { lane: 'execution', priority: 'Planning', provider_id: 'cloud-ai', model_id: 'planner-large', route_class: 'remote', reason: 'large context' },
  ],
  lane_fitness: [
    { lane: 'interactive', provider_id: 'edge-controller', avg_latency_ms: 18, success_count: 12, failure_count: 0 },
  ],
}

describe('genericControlSystemAdapter', () => {
  it('maps a non-ARDA fixture into the universal Fleet view model', () => {
    const model = createGenericFleetViewModel(fixture)

    expect(model.roleId).toBe('fleet')
    expect(model.title).toBe('Greenhouse Control Fleet')
    expect(model.status).toBe('attention')
    expect(model.metrics).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: 'systems_services', value: 3 }),
      expect.objectContaining({ id: 'providers', value: 2 }),
      expect.objectContaining({ id: 'healthy_providers', value: 2 }),
      expect.objectContaining({ id: 'health_attention', value: 2 }),
    ]))
    expect(model.providers[0]).toEqual(expect.objectContaining({
      providerId: 'edge-controller',
      providerName: 'Edge Controller',
      healthy: true,
      avgLatencyMs: 18,
    }))
    expect(model.laneOwnership[0]).toEqual(expect.objectContaining({
      lane: 'interactive',
      route: expect.objectContaining({ providerId: 'edge-controller', modelId: 'tiny-ops' }),
    }))
    expect(model.sources[0]).toEqual(expect.objectContaining({ id: 'greenhouse_manifest' }))
    expect(model.actions.map((action) => action.id)).toEqual(['refresh_greenhouse'])
    expect('raw' in model).toBe(false)
  })

  it('maps the same non-ARDA fixture into the universal Work view model', () => {
    const model = createGenericWorkViewModel(fixture)

    expect(model.roleId).toBe('work')
    expect(model.title).toBe('Greenhouse Control Work')
    expect(model.status).toBe('attention')
    expect(model.metrics).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: 'work_items', value: 3 }),
      expect.objectContaining({ id: 'active_work', value: 2 }),
      expect.objectContaining({ id: 'blocked_work', value: 1 }),
      expect.objectContaining({ id: 'pending_decisions', value: 1 }),
    ]))
    expect(model.sources[0].freshness.status).toBe('fresh')
    expect(model.actions.map((action) => action.id)).toEqual(['refresh_greenhouse'])
    expect('raw' in model).toBe(false)
  })

  it('returns safe empty view models for missing generic manifests', () => {
    expect(createGenericFleetViewModel(null)).toMatchObject({ roleId: 'fleet', status: 'empty', providers: [] })
    expect(createGenericWorkViewModel(undefined)).toMatchObject({ roleId: 'work', status: 'empty' })
  })

  it('produces the Fleet preview/focused contract consumed by boardroom components', () => {
    const model = createGenericFleetViewModel(fixture)
    const previewRoute = model.laneOwnership.find((lane) => lane.route)?.route
    const provider = model.providers.find((candidate) => candidate.enabled && candidate.healthy)

    expect(model.metrics.find((metric) => metric.id === 'healthy_providers')?.value).toBe(2)
    expect(previewRoute?.providerId).toBe('edge-controller')
    expect(provider?.models[0].id).toBe('tiny-ops')
    expect(model.summary.join(' ')).toMatch(/systems\/services registered/)
  })
})
