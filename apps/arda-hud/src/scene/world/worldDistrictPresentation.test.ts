// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { SceneZoneDefinition } from '../systems/runtimeTypes'
import type { WorldDistrictUrgency } from './worldDistrictUrgency'
import { resolveWorldDistrictPresentation } from './worldDistrictPresentation'

const zone: SceneZoneDefinition = {
  id: 'routing_and_comms',
  title: 'Routing & Comms',
  scene: 'world',
  owner: 'CHARON/HERMES',
  status: 'ok',
  anchorIds: ['routing-terminal'],
  surfaceIds: [],
  workstationIds: ['routing_and_comms-workstation'],
  sourceIds: ['routing_and_comms'],
}

function urgency(overrides: Partial<WorldDistrictUrgency> = {}): WorldDistrictUrgency {
  return {
    districtId: 'district_communications',
    sourceZoneId: 'routing_and_comms',
    urgency: 'critical',
    summary: 'Routing & Comms: critical; Critical/offline runtime signal.',
    topSignals: ['Critical/offline runtime signal'],
    recommendedAction: 'Open Routing workstation',
    ...overrides,
  }
}

describe('world district presentation', () => {
  it('binds urgency, action copy, and source zone to a clickable district label', () => {
    const result = resolveWorldDistrictPresentation(zone, urgency())

    expect(result.tone).toBe('critical')
    expect(result.color).toBe('#ff4d6d')
    expect(result.badge).toBe('critical')
    expect(result.detail).toContain('Critical/offline runtime signal')
    expect(result.openTargetZoneId).toBe('routing_and_comms')
    expect(result.title).toBe('Routing & Comms')
    expect(result.actionLabel).toBe('Open Routing workstation')
  })

  it('falls back to the scene zone when no urgency projection is available', () => {
    const result = resolveWorldDistrictPresentation(zone, undefined)

    expect(result.tone).toBe('unknown')
    expect(result.color).toBe('#7c879a')
    expect(result.detail).toContain('No urgency projection')
    expect(result.openTargetZoneId).toBe('routing_and_comms')
  })
})
