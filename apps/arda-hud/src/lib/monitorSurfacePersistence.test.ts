import { describe, expect, it, vi } from 'vitest'
import {
  MONITOR_SURFACE_REGISTRY_STORAGE_KEY,
  agentClaimMonitorSurface,
  agentGetMonitorSurfaceRegistry,
  agentReleaseMonitorSurface,
  agentRestoreMonitorSurfaceRegistry,
  loadPersistedMonitorSurfaceRegistry,
  persistMonitorSurfaceRegistry,
  rehydrateMonitorSurfaceRegistry,
} from './boardroomSlotSettings'

describe('monitor surface persistence', () => {
  it('persists and reloads a registry from localStorage', async () => {
    const registry = {
      schema_version: 'arda.monitor-session-registry.v2',
      updated_at_utc: '2026-08-07T10:00:00.000Z',
      sessions: {
        monitor_1: {
          slot_id: 'monitor_1',
          session_id: 'session-1',
          surface_session_id: 'session-1',
          owner: 'agent:agent-web',
          kind: 'agent',
          revision: 1,
          opened_at_utc: '2026-08-07T10:00:00.000Z',
          lease_expires_at_utc: '2026-08-07T12:00:00.000Z',
          content: { kind: 'web', url: 'https://example.invalid', display: 'inline', sandboxProfile: 'default' },
          workstation_handoff: { session_id: 'session-1', mode: 'same_live_session' },
          created_at_utc: '2026-08-07T10:00:00.000Z',
          updated_at_utc: '2026-08-07T10:00:00.000Z',
        },
      },
    }

    await persistMonitorSurfaceRegistry(registry as any)
    const loaded = await loadPersistedMonitorSurfaceRegistry()
    expect(loaded).toEqual(registry)
  })

  it('persists and reloads all five canonical monitor sessions without collapsing ownership', async () => {
    const sessions = Object.fromEntries(Array.from({ length: 5 }, (_, index) => {
      const number = index + 1
      const slotId = `monitor_${number}`
      const sessionId = `session-${number}`
      return [slotId, {
        slot_id: slotId,
        session_id: sessionId,
        surface_session_id: sessionId,
        owner: `agent:agent-${number}`,
        kind: 'message',
        revision: number,
        opened_at_utc: '2026-08-07T10:00:00.000Z',
        lease_expires_at_utc: '2026-08-07T12:00:00.000Z',
        content: { kind: 'message', text: `monitor ${number}` },
        workstation_handoff: { session_id: sessionId, mode: 'same_live_session' },
        created_at_utc: '2026-08-07T10:00:00.000Z',
        updated_at_utc: '2026-08-07T10:00:00.000Z',
      }]
    }))
    const registry = {
      schema_version: 'arda.monitor-session-registry.v2',
      updated_at_utc: '2026-08-07T10:00:00.000Z',
      sessions,
    }

    await persistMonitorSurfaceRegistry(registry as any)
    const loaded = await loadPersistedMonitorSurfaceRegistry()

    expect(loaded).toEqual(registry)
    expect(Object.keys(loaded?.sessions ?? {})).toEqual([
      'monitor_1', 'monitor_2', 'monitor_3', 'monitor_4', 'monitor_5',
    ])
    expect(Object.values(loaded?.sessions ?? {}).map((session) => session.owner))
      .toEqual(['agent:agent-1', 'agent:agent-2', 'agent:agent-3', 'agent:agent-4', 'agent:agent-5'])
  })

  it('rejects invalid persisted schema versions', async () => {
    const raw = { schema_version: 'arda.monitor-session-registry.v0', sessions: {} }
    // @ts-expect-error test invalid schema
    await persistMonitorSurfaceRegistry(raw)
    expect(await loadPersistedMonitorSurfaceRegistry()).toBeNull()
  })

  it('returns null when no registry is persisted', async () => {
    expect(await loadPersistedMonitorSurfaceRegistry()).toBeNull()
  })
})
