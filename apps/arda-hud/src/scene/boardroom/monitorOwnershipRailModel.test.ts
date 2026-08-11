import { describe, expect, it } from 'vitest'
import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'
import type { BoardroomAgentClaim } from '../../lib/boardroomSlotSettings'
import { deriveMonitorOwnershipRail } from './monitorOwnershipRailModel'

const session: MonitorSurfaceSessionRecord = {
  slot_id: 'monitor_2',
  session_id: 'session-2',
  surface_session_id: 'surface-session-2',
  owner: 'agent:mithrandir',
  kind: 'terminal',
  revision: 4,
  opened_at_utc: '2026-08-08T02:00:00.000Z',
  lease_expires_at_utc: '2026-08-08T02:10:00.000Z',
  content: { kind: 'terminal', sessionId: 'term-2' },
  workstation_handoff: { session_id: 'surface-session-2', mode: 'same_live_session' },
  created_at_utc: '2026-08-08T02:00:00.000Z',
  updated_at_utc: '2026-08-08T02:04:00.000Z',
}

const claim: BoardroomAgentClaim = {
  owner: 'legacy-agent',
  activity_kind: 'agent_activity',
  payload_binding: 'legacy.stream',
  fallback_preview: { mode: 'agent_activity', refresh_ms: 1000, widgets: [] },
  lease_expires_at_utc: '2026-08-08T02:09:00.000Z',
}

describe('monitor ownership rail model', () => {
  it('keeps a typed session authoritative over a legacy claim', () => {
    const rail = deriveMonitorOwnershipRail(session, claim, '2026-08-08T02:05:00.000Z')

    expect(rail.source).toBe('session')
    expect(rail.owner).toBe('agent:mithrandir')
    expect(rail.contentKind).toBe('terminal')
    expect(rail.leaseState).toBe('healthy')
    expect(rail.occupied).toBe(true)
  })

  it('falls back to the active claim when no typed session exists', () => {
    const rail = deriveMonitorOwnershipRail(null, claim, '2026-08-08T02:05:00.000Z')

    expect(rail.source).toBe('claim')
    expect(rail.owner).toBe('legacy-agent')
    expect(rail.contentKind).toBe('agent_activity')
  })

  it('emits a quiet dormant rail when no agent owns the monitor', () => {
    expect(deriveMonitorOwnershipRail(null, null, '2026-08-08T02:05:00.000Z')).toMatchObject({
      occupied: false,
      source: 'idle',
      owner: null,
      leaseState: 'idle',
    })
  })

  it('distinguishes expiring and expired ownership without hiding it', () => {
    expect(deriveMonitorOwnershipRail(session, null, '2026-08-08T02:09:30.000Z').leaseState).toBe('expiring')
    expect(deriveMonitorOwnershipRail(session, null, '2026-08-08T02:10:01.000Z').leaseState).toBe('expired')
  })

  it('derives stable owner fingerprints and distinct agent colors', () => {
    const first = deriveMonitorOwnershipRail(session, null, '2026-08-08T02:05:00.000Z')
    const repeat = deriveMonitorOwnershipRail(session, null, '2026-08-08T02:05:00.000Z')
    const other = deriveMonitorOwnershipRail({ ...session, owner: 'agent:radagast' }, null, '2026-08-08T02:05:00.000Z')

    expect(first.fingerprint).toEqual(repeat.fingerprint)
    expect(first.fingerprint).toHaveLength(12)
    expect(first.fingerprint.some(Boolean)).toBe(true)
    expect(first.color).not.toBe(other.color)
  })
})
