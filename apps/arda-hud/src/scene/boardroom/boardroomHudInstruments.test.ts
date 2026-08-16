import { describe, expect, it } from 'vitest'
import { createDefaultBoardroomSlotSettings } from '../../lib/boardroomSlotSettings'
import {
  deriveFleetHudInstrument,
  deriveGovernanceHudInstrument,
  deriveHumanHudInstrument,
  deriveBoardroomHudInstruments,
  deriveKnowledgeHudInstrument,
  deriveQueueHudInstrument,
  deriveRoutingHudInstrument,
  previewPresetForSource,
  previewTitleForSource,
  resolveHudInstrumentTruthPresentation,
  resolveBoardroomHudInstrument,
  type BoardroomHudInstrumentMap,
} from './boardroomHudInstruments'

describe('deriveGovernanceHudInstrument', () => {
  it('renders incident and actionable-decision pressure instead of raw record volume', () => {
    const quietArchive = deriveGovernanceHudInstrument({ reviewItems: 90, pendingItems: 0, incidentItems: 0 })
    const pressured = deriveGovernanceHudInstrument({ reviewItems: 2, pendingItems: 2, incidentItems: 1 })

    expect(quietArchive.eyebrow).toBe('DECISION PRESSURE')
    expect(quietArchive.glyph).toBe('0/0')
    expect(pressured.glyph).toBe('2/1')
    expect(pressured.nodes.filter((node) => node.state === 'warn')).not.toHaveLength(0)
    expect(pressured.nodes.filter((node) => node.state === 'warn').length)
      .toBeGreaterThan(quietArchive.nodes.filter((node) => node.state === 'warn').length)
  })
})

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

  it('preserves fleet source path, observation time, and freshness for the preview surface', () => {
    const instrument = deriveFleetHudInstrument({
      liveTargets: 3,
      totalTargets: 3,
      routableProviders: 2,
      unexpectedOffline: 0,
      intentionalOffline: 0,
      source: {
        sourceId: 'operator-runtime',
        sourceLabel: 'Operator Runtime',
        sourcePaths: ['core/state/operator_runtime_status.json'],
        observedAtUtc: '2026-07-30T20:15:00Z',
        freshness: 'fresh',
        sourceKind: 'live',
        truthState: 'live',
      },
    })

    expect(instrument.source).toEqual({
      sourceId: 'operator-runtime',
      sourceLabel: 'Operator Runtime',
      sourcePaths: ['core/state/operator_runtime_status.json'],
      observedAtUtc: '2026-07-30T20:15:00Z',
      freshness: 'fresh',
      sourceKind: 'live',
      truthState: 'live',
    })
  })

  it('does not use provider inventory as Fleet topology nodes', () => {
    const instrument = deriveFleetHudInstrument({
      liveTargets: 0,
      totalTargets: 0,
      routableProviders: 12,
      unexpectedOffline: 0,
      intentionalOffline: 0,
    })

    expect(instrument.nodes).toHaveLength(6)
    expect(instrument.glyph).toBe('0/6')
  })
})

describe('boardroom HUD instruments', () => {
  it('keeps the continuity instrument concise while including business, personal, and missing-reference pressure', () => {
    const model = deriveHumanHudInstrument({
      documents: 1,
      notes: 0,
      businessItems: 3,
      personalItems: 2,
      missingReferences: 1,
    })

    expect(model.glyph).toBe('6/1')
    expect(model.nodes.filter((node) => node.state === 'warn')).not.toHaveLength(0)
    expect(model.title).toBe('Continuity')
  })
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
      monitor_1: fallback,
    }

    expect(resolveBoardroomHudInstrument(instruments, 'boardroom.monitor.left', 'monitor_1')).toBe(exact)
  })

  it('uses the persisted assignment slot when no scene-zone instrument exists', () => {
    const instruments: BoardroomHudInstrumentMap = { monitor_1: instrument }

    expect(resolveBoardroomHudInstrument(instruments, 'boardroom.monitor.left', 'monitor_1')).toBe(instrument)
  })
})

describe('Phase 3 source-backed slot adapters', () => {
  it('maps each compact preview to the persisted workstation meaning and preserves provenance', () => {
    const source = {
      sourceId: 'test-source',
      sourceLabel: 'Test Source',
      sourcePaths: ['core/state/test.json'],
      observedAtUtc: '2026-07-30T20:15:00Z',
      freshness: 'fresh' as const,
      sourceKind: 'live' as const,
      truthState: 'live' as const,
    }
    const assignments = createDefaultBoardroomSlotSettings('2026-08-03T00:00:00Z').assignments
    const instruments = deriveBoardroomHudInstruments({
      fleetHealth: { liveTargets: 2, totalTargets: 2, routableProviders: 1, unexpectedOffline: 0, intentionalOffline: 0, source },
      queue: { completed: 4, priorityBuckets: 2, ownerBuckets: 2, source },
      knowledge: { documents: 8, plans: 3, source },
      routing: { routableProviders: 3, activeConnections: 2, constrainedHeadroom: 0.5, source },
      governance: { reviewItems: 4, pendingItems: 2, source },
      human: { documents: 5, notes: 2, source },
      dailyCommand: { lanes: 4, attentionLanes: 1, source },
    }, assignments)

    expect(instruments.monitor_2.title).toBe('Routing')
    expect(instruments.monitor_3.title).toBe('Knowledge')
    expect(instruments.monitor_4.title).toBe('Operations')
    expect(instruments.view_desk_l.title).toBe('Governance')
    expect(instruments.view_desk_control_panel.title).toBe('Fleet')
    expect(instruments.view_desk_r.title).toBe('Routing')
    expect(instruments.view_desk_aux.title).toBe('Continuity')
    expect(instruments.command_core.title).toBe('Daily Command')
    expect(Object.values(instruments).every((instrument) => instrument.source === source)).toBe(true)
  })

  it('does not label a sourced preview nominal when its provenance is missing', () => {
    const instrument = deriveQueueHudInstrument({
      completed: 4,
      priorityBuckets: 2,
      ownerBuckets: 2,
      source: {
        sourceId: 'queue',
        sourceLabel: 'Queue',
        sourcePaths: ['core/state/queue_summary.json'],
        observedAtUtc: null,
        freshness: 'missing',
        sourceKind: null,
        truthState: 'missing',
      },
    })

    expect(instrument.status).toBe('offline')
    expect(instrument.tone).toBe('rose')
  })

  it.each([
    ['blocked', 'offline'],
    ['unknown', 'offline'],
    ['stale', 'watch'],
  ] as const)('maps %s source provenance to %s rather than nominal', (freshness, expectedStatus) => {
    const instrument = deriveQueueHudInstrument({
      completed: 4,
      priorityBuckets: 2,
      ownerBuckets: 2,
      source: {
        sourceId: 'planning:core/state/queue_summary.json',
        sourceLabel: 'Queue Summary',
        sourceIds: ['planning:core/state/queue_summary.json'],
        sourcePaths: ['core/state/queue_summary.json'],
        observedAtUtc: freshness === 'unknown' ? null : '2026-07-30T20:15:00Z',
        freshness,
        sourceKind: 'snapshot',
        truthState: freshness === 'stale' ? 'stale' : 'unavailable',
      },
    })

    expect(instrument.status).toBe(expectedStatus)
    expect(instrument.status).not.toBe('nominal')
  })

  it.each([
    ['live', '●', 'LIVE'],
    ['snapshot', '□', 'SNAPSHOT'],
    ['projected', '◇', 'PROJECTED'],
    ['stale', '!', 'STALE'],
    ['unavailable', '×', 'UNAVAILABLE'],
    ['missing', '?', 'MISSING'],
  ] as const)('gives %s a deterministic non-color cue', (truthState, marker, label) => {
    expect(resolveHudInstrumentTruthPresentation(truthState)).toEqual({ marker, label })
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
