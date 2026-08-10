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
