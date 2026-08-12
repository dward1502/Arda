import { describe, expect, it } from 'vitest'
import type { MonitorSessionRegistryDescriptor } from '../../lib/monitorSurfaceContract'
import {
  createNativeAcceptanceSessions,
  serializeAcceptanceOwner,
  toClaimRequest,
  verifyNativeAcceptanceRegistry,
} from './nativeMonitorAcceptanceModel'

const projection = {
  schema_version: 'arda.operator-projection.v1' as const,
  projection_id: 'projection-native-acceptance',
  generated_at: '2026-08-11T21:00:00Z',
  authority: 'read_only' as const,
  freshness: 'fresh' as const,
  objectives: [],
  runs: [],
  capabilities: [],
  pending_approvals: [],
  councils: [],
  personal_operations: { captures: 0, resumable_items: 0, reminders: [] },
  joulework: {
    budget_joules: 0,
    consumed_joules: 0,
    remaining_joules: 0,
    source: 'unknown' as const,
    source_confidence: 0,
  },
  evidence: [],
  communications: [],
  dependencies: [],
}

describe('native monitor acceptance model', () => {
  it('defines five distinct owners and renderer classes on canonical slots', () => {
    const sessions = createNativeAcceptanceSessions(projection)

    expect(sessions.map(({ slotId }) => slotId)).toEqual([
      'monitor_1', 'monitor_2', 'monitor_3', 'monitor_4', 'monitor_5',
    ])
    expect(new Set(sessions.map(({ owner }) => serializeAcceptanceOwner(owner))).size).toBe(5)
    expect(sessions.map(({ content }) => content.kind)).toEqual([
      'document', 'image', 'video', 'terminal', 'component',
    ])
    expect(sessions.map(toClaimRequest).every(({ ttlMs }) => ttlMs === 3_600_000)).toBe(true)
  })

  it('requires exact ownership, content, and same-session handoffs', () => {
    const sessions = createNativeAcceptanceSessions(projection)
    const registry: MonitorSessionRegistryDescriptor = {
      schema_version: 'arda.monitor-session-registry.v2',
      updated_at_utc: '2026-08-11T21:00:00Z',
      sessions: Object.fromEntries(sessions.map((session, index) => {
        const surfaceSessionId = `surface-${index + 1}`
        return [session.slotId, {
          slot_id: session.slotId,
          session_id: surfaceSessionId,
          surface_session_id: surfaceSessionId,
          owner: serializeAcceptanceOwner(session.owner),
          kind: session.content.kind,
          revision: 1,
          opened_at_utc: '2026-08-11T21:00:00Z',
          lease_expires_at_utc: '2026-08-11T22:00:00Z',
          content: session.content,
          workstation_handoff: { session_id: surfaceSessionId, mode: 'same_live_session' },
          created_at_utc: '2026-08-11T21:00:00Z',
          updated_at_utc: '2026-08-11T21:00:00Z',
        }]
      })),
    }

    expect(verifyNativeAcceptanceRegistry(registry, sessions)).toEqual({
      ok: true,
      detail: 'sessions=5; owners=5; handoffs=same_live_session',
    })
    registry.sessions.monitor_3.workstation_handoff.session_id = 'wrong-session'
    expect(verifyNativeAcceptanceRegistry(registry, sessions)).toEqual({
      ok: false,
      detail: 'monitor_3:handoff-mismatch',
    })
  })
})
