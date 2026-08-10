import { describe, expect, it, vi } from 'vitest'
import type { MonitorSessionRegistryDescriptor } from './monitorSurfaceContract'
import {
  createEmptyMonitorRegistryBySlot,
  coerceRuntimeMonitorRegistry,
  createMonitorSurfaceRegistryBridge,
  selectActiveMonitorRecords,
  selectMonitorSlotContent,
  toRuntimeMonitorRegistry,
} from './monitorSurfaceRegistryBridge'

const registry: MonitorSessionRegistryDescriptor = {
  schema_version: 'arda.monitor-session-registry.v2',
  updated_at_utc: '2026-08-07T10:00:00.000Z',
  sessions: {
    monitor_1: {
      slot_id: 'monitor_1',
      session_id: 'session-doc',
      surface_session_id: 'session-doc',
      owner: 'agent:doc',
      kind: 'agent',
      revision: 1,
      opened_at_utc: '2026-08-07T10:00:00.000Z',
      lease_expires_at_utc: '2026-08-07T11:00:00.000Z',
      content: { kind: 'document', documentKind: 'markdown', source: { kind: 'local', path: 'docs/runbook.md' } },
      workstation_handoff: { session_id: 'session-doc', mode: 'same_live_session' },
      created_at_utc: '2026-08-07T10:00:00.000Z',
      updated_at_utc: '2026-08-07T10:00:00.000Z',
    },
    monitor_2: {
      slot_id: 'monitor_2',
      session_id: 'session-terminal',
      surface_session_id: 'session-terminal',
      owner: 'agent:terminal',
      kind: 'agent',
      revision: 2,
      opened_at_utc: '2026-08-07T10:00:00.000Z',
      lease_expires_at_utc: '2026-08-07T11:00:00.000Z',
      content: { kind: 'terminal', sessionId: 'named-main', readOnly: true },
      workstation_handoff: { session_id: 'session-terminal', mode: 'same_live_session' },
      created_at_utc: '2026-08-07T10:00:00.000Z',
      updated_at_utc: '2026-08-07T10:05:00.000Z',
    },
  },
}

describe('monitor surface registry bridge', () => {
  it('normalizes the camelCase typed Tauri registry without losing descriptor fields', () => {
    const converted = coerceRuntimeMonitorRegistry({
      schemaVersion: 'arda.monitor-session-registry.v2',
      updatedAtUtc: registry.updated_at_utc,
      sessions: {
        monitor_1: {
          slotId: 'monitor_1',
          sessionId: 'session-doc',
          surfaceSessionId: 'session-doc',
          owner: 'agent:doc',
          kind: 'agent',
          revision: 1,
          openedAtUtc: '2026-08-07T10:00:00.000Z',
          leaseExpiresAtUtc: '2026-08-07T11:00:00.000Z',
          content: registry.sessions.monitor_1.content,
          workstationHandoff: { sessionId: 'session-doc', mode: 'same_live_session' },
          createdAtUtc: '2026-08-07T10:00:00.000Z',
          updatedAtUtc: '2026-08-07T10:00:00.000Z',
        },
      },
    })

    expect(converted?.sessions.monitor_1.content).toEqual(registry.sessions.monitor_1.content)
    expect(converted?.sessions.monitor_1.workstation_handoff.session_id).toBe('session-doc')
    expect(coerceRuntimeMonitorRegistry(toRuntimeMonitorRegistry(registry))).toEqual(registry)
  })

  it('exposes every canonical monitor slot even when registry is empty', () => {
    expect(Object.keys(createEmptyMonitorRegistryBySlot())).toEqual([
      'monitor_1',
      'monitor_2',
      'monitor_3',
      'monitor_4',
      'monitor_5',
    ])
    expect(createEmptyMonitorRegistryBySlot().monitor_3).toBeNull()
  })

  it('prefers typed content and preserves legacy fallback only for a missing record', () => {
    expect(selectMonitorSlotContent(registry.sessions.monitor_1, 'legacy')).toEqual({
      descriptor: registry.sessions.monitor_1.content,
      legacy: null,
    })
    expect(selectMonitorSlotContent(null, 'legacy')).toEqual({ descriptor: null, legacy: 'legacy' })
  })

  it('selects five independent active slot records and filters expired or unknown slots', () => {
    const selected = selectActiveMonitorRecords({
      ...registry,
      sessions: {
        ...registry.sessions,
        monitor_3: {
          ...registry.sessions.monitor_1,
          slot_id: 'monitor_3',
          session_id: 'expired',
          surface_session_id: 'expired',
          lease_expires_at_utc: '2026-08-07T09:00:00.000Z',
        },
        unknown: {
          ...registry.sessions.monitor_1,
          slot_id: 'monitor_1',
          session_id: 'unknown',
          surface_session_id: 'unknown',
        },
      },
    }, '2026-08-07T10:30:00.000Z')

    expect(selected.monitor_1?.content.kind).toBe('document')
    expect(selected.monitor_2?.content.kind).toBe('terminal')
    expect(selected.monitor_3).toBeNull()
    expect(selected.monitor_4).toBeNull()
    expect(selected.monitor_5).toBeNull()
  })

  it('fetches the registry once, subscribes once, and replaces state from full typed events', async () => {
    const invoke = vi.fn().mockResolvedValue(registry)
    const unlisten = vi.fn()
    const listen = vi.fn().mockImplementation((_event, handler) => {
      handler({ payload: { registry: { ...registry, sessions: { monitor_5: { ...registry.sessions.monitor_1, slot_id: 'monitor_5' } } } } })
      return Promise.resolve(unlisten)
    })
    const onRegistry = vi.fn()
    const bridge = createMonitorSurfaceRegistryBridge({ invoke, listen, onRegistry })

    await bridge.start()
    await bridge.stop()

    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('get_monitor_surface_registry')
    expect(listen).toHaveBeenCalledTimes(1)
    expect(listen).toHaveBeenCalledWith('monitor-surface-registry-changed', expect.any(Function))
    expect(onRegistry).toHaveBeenCalledTimes(2)
    expect(onRegistry.mock.calls[0][0].sessions.monitor_1.content.kind).toBe('document')
    expect(onRegistry.mock.calls[1][0].sessions.monitor_5.content.kind).toBe('document')
    expect(unlisten).toHaveBeenCalledTimes(1)
  })

  it('degrades safely without Tauri invoke/listen bindings', async () => {
    const onRegistry = vi.fn()
    const bridge = createMonitorSurfaceRegistryBridge({ onRegistry })

    await expect(bridge.start()).resolves.toEqual(createEmptyMonitorRegistryBySlot())
    await expect(bridge.stop()).resolves.toBeUndefined()
    expect(onRegistry).not.toHaveBeenCalled()
  })

  it('still subscribes when the one-time startup fetch fails', async () => {
    const listen = vi.fn().mockResolvedValue(vi.fn())
    const bridge = createMonitorSurfaceRegistryBridge({
      invoke: vi.fn().mockRejectedValue(new Error('startup unavailable')),
      listen,
    })

    await expect(bridge.start()).resolves.toEqual(createEmptyMonitorRegistryBySlot())
    expect(listen).toHaveBeenCalledTimes(1)
  })
})
