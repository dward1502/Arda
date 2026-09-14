import { describe, expect, it } from 'vitest'
import { lifecyclePrimaryAction, lifecycleRows, type LifecycleSnapshot } from './lifecycle'

const snapshot = (state: LifecycleSnapshot['aggregate_state']): LifecycleSnapshot => ({
  schema_version: 'arda.system-lifecycle.v1',
  observed_at: '2026-08-17T00:00:00Z',
  aggregate_state: state,
  components: [{
    component_id: 'arda-runtime', class: 'required',
    unit: { owning_unit: 'arda.service', enablement: { value: 'enabled', observation: { source: 'systemd', source_id: 'arda.service', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } }, active_state: { value: 'active', observation: { source: 'systemd', source_id: 'arda.service', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } }, sub_state: { value: 'running', observation: { source: 'systemd', source_id: 'arda.service', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } } },
    protocol_health: { value: 'healthy', observation: { source: 'protocol_probe', source_id: 'health', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } }, diagnostic: null, recovery_action: 'retry-health-check',
  }],
  hud_native: { availability: { value: 'available', observation: { source: 'filesystem', source_id: 'hud', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } }, running: { value: 'stopped', observation: { source: 'systemd', source_id: 'hud', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } } },
  hermes_gateway: { availability: { value: 'available', observation: { source: 'systemd', source_id: 'gateway', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } }, protocol_health: { value: 'healthy', observation: { source: 'protocol_probe', source_id: 'gateway', observed_at: '2026-08-17T00:00:00Z', freshness: 'fresh' } }, },
})

describe('lifecycle presentation', () => {
  it.each([
    ['stopped', 'start'], ['starting', 'inspect'], ['healthy', 'open_hud'],
    ['degraded', 'retry'], ['failed', 'inspect'], ['stopping', 'inspect'], ['unknown', 'inspect'],
  ] as const)('maps %s to %s', (state, action) => {
    expect(lifecyclePrimaryAction(snapshot(state)).kind).toBe(action)
  })

  it('turns stale required evidence into retry', () => {
    const value = snapshot('healthy')
    value.components[0].protocol_health.observation.freshness = 'stale'
    expect(lifecyclePrimaryAction(value).kind).toBe('retry')
  })

  it('projects concise required component truth', () => {
    expect(lifecycleRows(snapshot('healthy'))).toEqual([
      { id: 'arda-runtime', process: 'active', health: 'healthy', freshness: 'fresh', recovery: 'retry-health-check' },
    ])
  })
})
