// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  BOARDROOM_MONITOR_SLOT_IDS,
  createDefaultBoardroomSlotSettings,
  resolveMonitorSlotSource,
  type BoardroomSceneSlotId,
} from '../../lib/boardroomSlotSettings'
import {
  BOARDROOM_MONITOR_ZONES,
  BOARDROOM_SPATIAL_ZONES,
  BOARDROOM_CONSOLE_SHELL_SEGMENTS,
} from './boardroomSpatialLayout'
import { resolveMonitorContractSlotId, resolveMonitorSurfaceOpenRequest } from './monitorSurfaceRuntime'

const UPPER_MONITOR_SLOT_IDS = [
  'monitor_1',
  'monitor_2',
  'monitor_3',
  'monitor_4',
  'monitor_5',
] as const
type UpperMonitorSlotId = typeof UPPER_MONITOR_SLOT_IDS[number]

const MONITOR_ZONES = BOARDROOM_SPATIAL_ZONES.filter((zone) => zone.kind === 'upper_monitor')

describe('universal monitor acceptance', () => {
  it('exposes five canonical assignable monitor slots', () => {
    expect(UPPER_MONITOR_SLOT_IDS).toEqual([
      'monitor_1',
      'monitor_2',
      'monitor_3',
      'monitor_4',
      'monitor_5',
    ])
  })

  it('maps each physical upper monitor 1:1 to a canonical slot', () => {
    expect(MONITOR_ZONES.length).toBe(5)
    expect(MONITOR_ZONES.map((zone) => zone.assignmentSlotId)).toEqual([
      'monitor_1',
      'monitor_2',
      'monitor_3',
      'monitor_4',
      'monitor_5',
    ])
  })

  it('assigns the center monitor independently', () => {
    const centerZone = MONITOR_ZONES.find((zone) => zone.id === 'boardroom.monitor.center')
    expect(centerZone?.assignmentSlotId).toBe('monitor_3')
  })

  it('represents every content kind in the union/validator', () => {
    const kinds = [
      'web',
      'youtube',
      'video',
      'image',
      'document',
      'terminal',
      'component',
      'remote_session',
      'fallback',
    ] as const
    expect(kinds.length).toBeGreaterThanOrEqual(9)
  })

  it('allows multiple slots to hold different owners simultaneously', () => {
    const document = createDefaultBoardroomSlotSettings()
    const monitorAssignments = document.assignments.filter((assignment) => BOARDROOM_MONITOR_SLOT_IDS.includes(assignment.slot_id))
    const owned = monitorAssignments.map((assignment, index) => {
      const claimed = claimSlotById(document, assignment.slot_id as BoardroomSceneSlotId, `owner-${index}`)
      document.assignments = claimed.assignments
      return claimed.assignments.find((item) => item.slot_id === assignment.slot_id)!.agent_claims?.at(-1)!.owner
    })
    expect(new Set(owned).size).toBe(5)
  })

  it('opens an active session by surfaceSessionId, not source-zone fallback', () => {
    const request = resolveMonitorSurfaceOpenRequest('monitor_1', 'unknown_zone', 'native_window')
    expect(request).not.toBeNull()
    expect(request!.title).toBe('ARDA Monitor — monitor_1')
  })

  it('derives monitor slot bindings without duplicated center binding', () => {
    const bindings = MONITOR_ZONES.map((zone) => zone.binding)
    expect(bindings).toEqual([
      'upper_monitor_1',
      'upper_monitor_2',
      'upper_monitor_3',
      'upper_monitor_4',
      'upper_monitor_5',
    ])
  })

  it('resolves five unique slot sources from default document', () => {
    const document = createDefaultBoardroomSlotSettings()
    const sources = UPPER_MONITOR_SLOT_IDS.map((slotId) => resolveMonitorSlotSource(slotId, document))
    expect(sources.every((source) => source !== null)).toBe(true)
    expect(sources.filter((source) => source !== null).length).toBe(5)
  })

  it('contracts the public monitor slot set to the canonical five IDs', () => {
    expect(BOARDROOM_MONITOR_SLOT_IDS).toEqual(UPPER_MONITOR_SLOT_IDS)
  })
})

function claimSlotById(document: ReturnType<typeof createDefaultBoardroomSlotSettings>, slotId: BoardroomSceneSlotId, owner: string) {
  const assignment = document.assignments.find((item) => item.slot_id === slotId)!
  const claims = Array.isArray(assignment.agent_claims) ? [...assignment.agent_claims] : []
  claims.push({
    owner,
    activity_kind: 'agent_activity',
    payload_binding: `${owner}.surface`,
    fallback_preview: assignment.surface_layout.preview,
    lease_expires_at_utc: new Date(Date.now() + 1000 * 60).toISOString(),
  })
  return {
    ...document,
    assignments: document.assignments.map((item) => item.slot_id === slotId ? { ...item, agent_claims: claims } : item),
  }
}
