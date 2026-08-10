import {
  CANONICAL_MONITOR_SLOT_IDS,
  isMonitorSessionRecord,
  isSessionActive,
  parseMonitorSessionRegistry,
  type MonitorContentDescriptor,
  type MonitorSessionRegistryDescriptor,
  type MonitorSurfaceSessionRecord,
  type UpperMonitorSlotId,
} from './monitorSurfaceContract'

export type MonitorRecordsBySlot = Record<UpperMonitorSlotId, MonitorSurfaceSessionRecord | null>

type InvokeFn = <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>
type ListenFn = <T = unknown>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>

export interface MonitorSurfaceRegistryBridgeOptions {
  invoke?: InvokeFn
  listen?: ListenFn
  onRegistry?: (registry: MonitorSessionRegistryDescriptor) => void
  nowUtc?: () => string
}

export interface MonitorSurfaceRegistryBridge {
  start: () => Promise<MonitorRecordsBySlot>
  stop: () => Promise<void>
}

export function createEmptyMonitorRegistryBySlot(): MonitorRecordsBySlot {
  return CANONICAL_MONITOR_SLOT_IDS.reduce((records, slotId) => {
    records[slotId] = null
    return records
  }, {} as MonitorRecordsBySlot)
}

export function selectActiveMonitorRecords(
  registry: MonitorSessionRegistryDescriptor | null | undefined,
  nowUtc: string | number = Date.now(),
): MonitorRecordsBySlot {
  const records = createEmptyMonitorRegistryBySlot()
  if (!registry) return records
  const now = typeof nowUtc === 'number' ? nowUtc : new Date(nowUtc).getTime()
  for (const slotId of CANONICAL_MONITOR_SLOT_IDS) {
    const record = registry.sessions[slotId]
    if (!isMonitorSessionRecord(record)) continue
    if (record.slot_id !== slotId) continue
    if (!isSessionActive(record, now)) continue
    records[slotId] = record
  }
  return records
}

export function selectMonitorSlotContent<T>(
  record: MonitorSurfaceSessionRecord | null | undefined,
  legacy: T,
): { descriptor: MonitorContentDescriptor | null; legacy: T | null } {
  return record
    ? { descriptor: record.content, legacy: null }
    : { descriptor: null, legacy }
}

function registryFromEventPayload(payload: unknown): MonitorSessionRegistryDescriptor | null {
  const direct = coerceRuntimeMonitorRegistry(payload)
  if (direct) return direct
  if (payload && typeof payload === 'object' && !Array.isArray(payload)) {
    const record = payload as Record<string, unknown>
    return coerceRuntimeMonitorRegistry(record.registry)
  }
  return null
}

function asObject(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null
}

/** Convert the camelCase Tauri wire document into the canonical persisted TS contract. */
export function coerceRuntimeMonitorRegistry(value: unknown): MonitorSessionRegistryDescriptor | null {
  const canonical = parseMonitorSessionRegistry(value)
  if (canonical) return canonical
  const registry = asObject(value)
  const rawSessions = asObject(registry?.sessions)
  if (!registry || registry.schemaVersion !== 'arda.monitor-session-registry.v2' || !rawSessions) return null

  const sessions: Record<string, MonitorSurfaceSessionRecord> = {}
  for (const [slotId, value] of Object.entries(rawSessions)) {
    const record = asObject(value)
    const handoff = asObject(record?.workstationHandoff)
    if (!record || !handoff) continue
    const converted = {
      slot_id: record.slotId,
      session_id: record.sessionId,
      surface_session_id: record.surfaceSessionId,
      owner: record.owner,
      kind: record.kind,
      revision: record.revision,
      opened_at_utc: record.openedAtUtc,
      lease_expires_at_utc: record.leaseExpiresAtUtc,
      content: record.content,
      workstation_handoff: {
        session_id: handoff.sessionId,
        mode: handoff.mode,
      },
      created_at_utc: record.createdAtUtc,
      updated_at_utc: record.updatedAtUtc,
    }
    if (isMonitorSessionRecord(converted)) sessions[slotId] = converted
  }
  return parseMonitorSessionRegistry({
    schema_version: registry.schemaVersion,
    updated_at_utc: registry.updatedAtUtc,
    sessions,
  })
}

export function toRuntimeMonitorRegistry(registry: MonitorSessionRegistryDescriptor): Record<string, unknown> {
  return {
    schemaVersion: registry.schema_version,
    updatedAtUtc: registry.updated_at_utc,
    sessions: Object.fromEntries(Object.entries(registry.sessions).map(([slotId, record]) => [slotId, {
      slotId: record.slot_id,
      sessionId: record.session_id,
      surfaceSessionId: record.surface_session_id,
      owner: record.owner,
      kind: record.kind,
      revision: record.revision,
      openedAtUtc: record.opened_at_utc,
      leaseExpiresAtUtc: record.lease_expires_at_utc,
      content: record.content,
      workstationHandoff: {
        sessionId: record.workstation_handoff.session_id,
        mode: record.workstation_handoff.mode,
      },
      createdAtUtc: record.created_at_utc,
      updatedAtUtc: record.updated_at_utc,
    }])),
  }
}

export function createMonitorSurfaceRegistryBridge({
  invoke,
  listen,
  onRegistry,
  nowUtc = () => new Date().toISOString(),
}: MonitorSurfaceRegistryBridgeOptions): MonitorSurfaceRegistryBridge {
  let unlisten: (() => void) | null = null
  let stopped = false

  const applyRegistry = (registry: MonitorSessionRegistryDescriptor) => {
    if (!stopped) onRegistry?.(registry)
    return selectActiveMonitorRecords(registry, nowUtc())
  }

  return {
    async start() {
      stopped = false
      if (!invoke || !listen) return createEmptyMonitorRegistryBySlot()
      let selected = createEmptyMonitorRegistryBySlot()
      try {
        const runtimeRegistry = await invoke<unknown>('get_monitor_surface_registry')
        const parsed = coerceRuntimeMonitorRegistry(runtimeRegistry)
        if (parsed) selected = applyRegistry(parsed)
      } catch {
        // A transient startup read must not prevent later authoritative events.
      }
      try {
        unlisten = await listen<unknown>('monitor-surface-registry-changed', ({ payload }) => {
          const parsed = registryFromEventPayload(payload)
          if (parsed) applyRegistry(parsed)
        })
      } catch {
        unlisten = null
      }
      return selected
    },
    async stop() {
      stopped = true
      const dispose = unlisten
      unlisten = null
      dispose?.()
    },
  }
}
