import { describe, expect, it } from 'vitest'
import { createEmptyMonitorRegistryBySlot } from '../../lib/monitorSurfaceRegistryBridge'
import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'
import {
  createMonitorSessionWindowConfig,
  findMonitorSessionRecord,
  parseMonitorSessionWorkstationId,
} from './monitorSessionWorkstationRoute'

const record: MonitorSurfaceSessionRecord = {
  slot_id: 'monitor_3',
  session_id: 'session-3',
  surface_session_id: 'surface-session-3',
  owner: 'agent-gimli',
  kind: 'document',
  revision: 4,
  opened_at_utc: '2026-08-07T20:00:00.000Z',
  lease_expires_at_utc: '2026-08-07T21:00:00.000Z',
  content: { kind: 'document', documentKind: 'markdown', source: { kind: 'local', path: 'README.md' } },
  workstation_handoff: { session_id: 'surface-session-3', mode: 'same_live_session' },
  created_at_utc: '2026-08-07T20:00:00.000Z',
  updated_at_utc: '2026-08-07T20:01:00.000Z',
}

describe('monitor session workstation route', () => {
  it('uses one stable native window identity for repeated opens of the same surface session', () => {
    const first = createMonitorSessionWindowConfig(record)
    const second = createMonitorSessionWindowConfig({ ...record, revision: 5 })

    expect(first.id).toBe('monitor-workstation-surface-session-3')
    expect(second.id).toBe(first.id)
    expect(first.workstationId).toBe('monitor-session:surface-session-3')
  })

  it('parses only monitor-session workstation identities', () => {
    expect(parseMonitorSessionWorkstationId('monitor-session:surface-session-3')).toBe('surface-session-3')
    expect(parseMonitorSessionWorkstationId('ordinary-workstation')).toBeNull()
    expect(parseMonitorSessionWorkstationId('monitor-session:')).toBeNull()
  })

  it('selects the exact shared record rather than a slot-global singleton', () => {
    const records = createEmptyMonitorRegistryBySlot()
    records.monitor_3 = record
    expect(findMonitorSessionRecord(records, 'surface-session-3')).toEqual(record)
    expect(findMonitorSessionRecord(records, 'different-session')).toBeNull()
  })
})
