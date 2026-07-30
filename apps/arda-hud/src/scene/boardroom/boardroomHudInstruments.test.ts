import { describe, expect, it } from 'vitest'
import {
  deriveFleetHudInstrument,
  deriveKnowledgeHudInstrument,
  deriveQueueHudInstrument,
  deriveRoutingHudInstrument,
  previewPresetForSource,
  previewTitleForSource,
  resolveBoardroomHudInstrument,
  type BoardroomHudInstrumentMap,
} from './boardroomHudInstruments'

describe('deriveFleetHudInstrument', () => {
  it('creates a nominal fleet instrument when targets are live', () => {
    const instrument = deriveFleetHudInstrument({
      liveTargets: 5,
      totalTargets: 5,
      routableProviders: 3,
      unexpectedOffline: 0,
      intentionalOffline: 0,
      runtimeDrift: { driftedNodes: 0, totalNodes: 5 },
    })

    expect(instrument.status).toBe('nominal')
    expect(instrument.tone).toBe('cyan')
    expect(instrument.glyph).toBe('5/5')
    expect(instrument.nodes.filter((node) => node.state === 'good')).toHaveLength(5)
  })

  it('marks unexpected offline targets as alert nodes', () => {
    const instrument = deriveFleetHudInstrument({
      liveTargets: 3,
      totalTargets: 6,
      routableProviders: 2,
      unexpectedOffline: 2,
      intentionalOffline: 1,
      runtimeDrift: { driftedNodes: 1, totalNodes: 6 },
    })

    expect(instrument.status).toBe('watch')
    expect(instrument.tone).toBe('gold')
    expect(instrument.nodes.filter((node) => node.state === 'alert')).toHaveLength(2)
    expect(instrument.nodes.filter((node) => node.state === 'warn')).toHaveLength(1)
  })
})

describe('resolveBoardroomHudInstrument', () => {
  const instrument = deriveFleetHudInstrument({
    liveTargets: 2,
    totalTargets: 2,
    routableProviders: 1,
    unexpectedOffline: 0,
    intentionalOffline: 0,
  })

  it('prefers an exact scene-zone instrument over its assignment slot fallback', () => {
    const exact = { ...instrument, title: 'Exact zone' }
    const fallback = { ...instrument, title: 'Slot fallback' }
    const instruments: BoardroomHudInstrumentMap = {
      'boardroom.monitor.left': exact,
      monitor_left_1: fallback,
    }

    expect(resolveBoardroomHudInstrument(instruments, 'boardroom.monitor.left', 'monitor_left_1')).toBe(exact)
  })

  it('uses the persisted assignment slot when no scene-zone instrument exists', () => {
    const instruments: BoardroomHudInstrumentMap = { monitor_left_1: instrument }

    expect(resolveBoardroomHudInstrument(instruments, 'boardroom.monitor.left', 'monitor_left_1')).toBe(instrument)
  })
})

describe('Phase 1 preview presets', () => {
  it('uses meaningfully different visual grammars for live source families', () => {
    expect(deriveFleetHudInstrument({
      liveTargets: 3,
      totalTargets: 4,
      routableProviders: 2,
      unexpectedOffline: 1,
      intentionalOffline: 0,
    }).preset).toBe('topology')
    expect(deriveRoutingHudInstrument({
      routableProviders: 3,
      activeConnections: 2,
      constrainedHeadroom: 0.5,
    }).preset).toBe('routes')
    expect(deriveQueueHudInstrument({
      completed: 5,
      priorityBuckets: 2,
      ownerBuckets: 3,
    }).preset).toBe('lanes')
    expect(deriveKnowledgeHudInstrument({ documents: 8, plans: 2 }).preset).toBe('constellation')
  })

  it('derives stable fallback presets from persisted workstation source IDs', () => {
    expect(previewPresetForSource('routing_and_comms')).toBe('routes')
    expect(previewPresetForSource('planning_and_queue')).toBe('lanes')
    expect(previewPresetForSource('memory_and_continuity')).toBe('constellation')
    expect(previewPresetForSource('sovereign_world')).toBe('pulse')
    expect(previewPresetForSource('systems_health')).toBe('topology')
    expect(previewPresetForSource('service_warp_dev')).toBe('standby')
  })

  it('keeps persisted source identity legible when workstation metadata is unavailable', () => {
    expect(previewTitleForSource('routing_and_comms')).toBe('Routing + Comms')
    expect(previewTitleForSource('memory_and_continuity')).toBe('Memory + Continuity')
    expect(previewTitleForSource('planning_and_queue')).toBe('Planning + Queue')
    expect(previewTitleForSource('service_warp_dev')).toBe('Warp Dev')
  })
})
