// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { ArdaSourceProvenance } from '../../lib/ardaProvenance'
import type { ArdaSection } from '../../lib/ardaSource'
import type { SceneZoneDefinition } from '../systems/runtimeTypes'
import { getWorldDistrictContract } from './worldDistrictContracts'
import { calculateWorldDistrictUrgencies, calculateWorldDistrictUrgency } from './worldDistrictUrgency'

function provenance(domainId: string, state: ArdaSourceProvenance['state'] = 'fresh'): ArdaSourceProvenance {
  return {
    domainId,
    label: domainId,
    sourcePaths: [domainId.split(':')[1] ?? domainId],
    generatedAtUtc: '2026-05-22T00:00:00.000Z',
    observedAtUtc: '2026-05-22T00:00:00.000Z',
    state,
    sourceKind: 'snapshot',
  }
}

function section(id: string, extra: Record<string, unknown> = {}): ArdaSection {
  return {
    id,
    title: id,
    owner: 'TEST',
    status: 'ok',
    arda_panels: [],
    primary_sources: [],
    ...extra,
  } as unknown as ArdaSection
}

describe('world district urgency', () => {
  it('maps stale provenance to stale urgency', () => {
    const contract = getWorldDistrictContract('district_command')!
    const result = calculateWorldDistrictUrgency(contract, {
      sections: [section(contract.sourceZoneId)],
      sourceProvenance: [provenance('world:core/realm/agents.toml', 'stale')],
    })
    expect(result.urgency).toBe('stale')
    expect(result.topSignals[0]).toContain('freshness')
  })

  it('maps blocked governance signals to blocked urgency', () => {
    const contract = getWorldDistrictContract('district_governance')!
    const result = calculateWorldDistrictUrgency(contract, {
      sections: [section(contract.sourceZoneId, { highlights: ['triad required: autonomy blocked'] })],
      sourceProvenance: [provenance('governance:core/realm/arda.toml')],
    })
    expect(result.urgency).toBe('blocked')
  })

  it('maps provider failures to critical routing pressure', () => {
    const contract = getWorldDistrictContract('district_communications')!
    const result = calculateWorldDistrictUrgency(contract, {
      sections: [section(contract.sourceZoneId, { alerts: ['provider unhealthy and routing failed'] })],
      sourceProvenance: [provenance('routing:config/charon_providers.toml')],
    })
    expect(result.urgency).toBe('critical')
  })

  it('maps open queue depth to attention pressure', () => {
    const contract = getWorldDistrictContract('district_operations')!
    const result = calculateWorldDistrictUrgency(contract, {
      sections: [section(contract.sourceZoneId, { counts: { open_tasks_total: 4 } })],
      sourceProvenance: [provenance('planning:core/projects/tasks/queue.jsonl')],
    })
    expect(result.urgency).toBe('attention')
    expect(result.topSignals).toContain('Queue pressure: 4')
  })

  it('returns unknown for unmapped world zones', () => {
    const zones: SceneZoneDefinition[] = [{
      id: 'district_unknown',
      title: 'Unknown District',
      scene: 'world',
      owner: 'NONE',
      status: 'unknown',
      anchorIds: [],
      surfaceIds: [],
      workstationIds: [],
      sourceIds: [],
    }]
    const result = calculateWorldDistrictUrgencies(zones, { sections: [], sourceProvenance: [] })
    expect(result.district_unknown.urgency).toBe('unknown')
    expect(result.district_unknown.recommendedAction).toBe('Define district contract')
  })
})
