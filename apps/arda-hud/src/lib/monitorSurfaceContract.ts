export type MonitorContentDescriptor =
  | { kind: 'web'; url: string; title?: string; display: 'inline' | 'capture_stream'; sandboxProfile: string }
  | { kind: 'youtube'; videoId: string; startSeconds?: number; autoplay?: boolean; muted?: boolean }
  | { kind: 'video'; source: MonitorMediaSource; mime?: string; fit: 'contain' | 'cover'; loop?: boolean; autoplay?: boolean; muted?: boolean }
  | { kind: 'image'; source: MonitorMediaSource; fit: 'contain' | 'cover'; alt?: string }
  | { kind: 'document'; source: MonitorMediaSource; documentKind: 'pdf' | 'markdown' | 'text'; page?: number }
  | { kind: 'terminal'; sessionId: string; readOnly?: boolean; theme?: string }
  | { kind: 'component'; rendererId: string; props: Record<string, unknown> }
  | { kind: 'remote_session'; sessionId: string; streamUrl: string; transport: 'webrtc' | 'hls' | 'mjpeg' }
  | { kind: 'fallback'; reason: string; retryable: boolean }

export type MonitorMediaSource =
  | { kind: 'local'; path: string }
  | { kind: 'remote'; url: string }

export interface MonitorPlaybackState {
  playing: boolean
  currentTime: number
  duration?: number
  volume?: number
}

export interface MonitorNavigationState {
  url?: string
  title?: string
  scrollY?: number
}

export interface MonitorSurfaceSession {
  schemaVersion: 'arda.monitor-surface-session.v2'
  surfaceSessionId: string
  slotId: UpperMonitorSlotId
  owner: string
  content: MonitorContentDescriptor
  revision: number
  leaseExpiresAtUtc: string
  createdAtUtc: string
  updatedAtUtc: string
  playback?: MonitorPlaybackState
  navigation?: MonitorNavigationState
}

export type AgentSurfaceOwner =
  | { kind: 'agent'; name: string }
  | { kind: 'operator'; id: string }
  | { kind: 'system'; name: string }

export interface MonitorSurfaceClaimRequest {
  slotId: UpperMonitorSlotId
  owner: AgentSurfaceOwner
  initialContent: MonitorContentDescriptor
  ttlMs: number
}

export interface MonitorSurfaceUpdateRequest {
  surfaceSessionId: string
  owner: AgentSurfaceOwner
  content: MonitorContentDescriptor
  expectedRevision: number
}

export interface MonitorSurfacePatchRequest {
  surfaceSessionId: string
  owner: AgentSurfaceOwner
  playback?: MonitorPlaybackState
  navigation?: MonitorNavigationState
  expectedRevision: number
}

export interface MonitorSurfaceRefreshRequest {
  surfaceSessionId: string
  owner: AgentSurfaceOwner
  ttlMs: number
}

export interface MonitorSurfaceReleaseRequest {
  surfaceSessionId: string
  owner: AgentSurfaceOwner
}

export interface MonitorSurfaceSessionRecord {
  slot_id: UpperMonitorSlotId
  session_id: string
  surface_session_id: string
  owner: string
  kind: string
  revision: number
  opened_at_utc: string
  lease_expires_at_utc: string
  content: MonitorContentDescriptor
  playback?: MonitorPlaybackState
  workstation_handoff: WorkstationHandoffDescriptor
  created_at_utc: string
  updated_at_utc: string
}

export interface WorkstationHandoffDescriptor {
  session_id: string
  mode: 'same_live_session' | 'independent_window' | 'unsupported'
}

export interface MonitorSessionClaimResult {
  ok: boolean
  registry: MonitorSessionRegistryDescriptor
  session: MonitorSurfaceSessionRecord | null
  message: string
}

export interface MonitorSessionRegistryDescriptor {
  schema_version: 'arda.monitor-session-registry.v2'
  updated_at_utc: string
  sessions: Record<string, MonitorSurfaceSessionRecord>
}

export const MONITOR_SESSION_REGISTRY_SCHEMA_VERSION =
  'arda.monitor-session-registry.v2'
export const MONITOR_SURFACE_SCHEMA_VERSION = 'arda.monitor-surface-session.v2'
export const CANONICAL_MONITOR_SLOT_IDS = [
  'monitor_1',
  'monitor_2',
  'monitor_3',
  'monitor_4',
  'monitor_5',
] as const
export type UpperMonitorSlotId = typeof CANONICAL_MONITOR_SLOT_IDS[number]

export function isUpperMonitorSlotId(value: unknown): value is UpperMonitorSlotId {
  return typeof value === 'string' && CANONICAL_MONITOR_SLOT_IDS.includes(value as UpperMonitorSlotId)
}

export function createEmptyMonitorSessionRegistry(
  updatedAtUtc = new Date(0).toISOString(),
): MonitorSessionRegistryDescriptor {
  return {
    schema_version: MONITOR_SESSION_REGISTRY_SCHEMA_VERSION,
    updated_at_utc: updatedAtUtc,
    sessions: {},
  }
}

export function parseMonitorSessionRegistry(
  value: unknown,
): MonitorSessionRegistryDescriptor | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const record = value as Record<string, unknown>
  if (record.schema_version !== MONITOR_SESSION_REGISTRY_SCHEMA_VERSION) return null
  const sessions: Record<string, MonitorSurfaceSessionRecord> = {}
  const rawSessions =
    record.sessions && typeof record.sessions === 'object' && !Array.isArray(record.sessions)
      ? (record.sessions as Record<string, unknown>)
      : {}
  for (const [slotId, raw] of Object.entries(rawSessions)) {
    if (!isMonitorSessionRecord(raw)) continue
    sessions[slotId] = raw
  }
  return {
    schema_version: MONITOR_SESSION_REGISTRY_SCHEMA_VERSION,
    updated_at_utc:
      typeof record.updated_at_utc === 'string' ? record.updated_at_utc : new Date(0).toISOString(),
    sessions,
  }
}

export function createMonitorSessionClaim(
  registry: MonitorSessionRegistryDescriptor,
  descriptor: MonitorSurfaceClaimRequest,
  nowUtc = new Date().toISOString(),
): MonitorSessionClaimResult {
  if (!isUpperMonitorSlotId(descriptor.slotId)) {
    return {
      ok: false,
      registry,
      session: null,
      message: `slotId '${descriptor.slotId}' is not a canonical monitor slot`,
    }
  }
  const existing = registry.sessions[descriptor.slotId]
  const now = new Date(nowUtc).getTime()
  const existingActive = existing ? new Date(existing.lease_expires_at_utc).getTime() > now : false
  if (existingActive && existing.owner !== serializeOwner(descriptor.owner)) {
    return {
      ok: false,
      registry,
      session: existing,
      message: `Session for '${descriptor.slotId}' is already owned by '${existing.owner}'`,
    }
  }
  const surfaceSessionId = existing?.surface_session_id ?? `session-${descriptor.slotId}-${Date.now()}`
  const session: MonitorSurfaceSessionRecord = {
    slot_id: descriptor.slotId,
    session_id: surfaceSessionId,
    surface_session_id: surfaceSessionId,
    owner: serializeOwner(descriptor.owner),
    kind: descriptor.owner.kind,
    revision: existing?.revision ? existing.revision + 1 : 1,
    opened_at_utc: existing?.opened_at_utc ?? nowUtc,
    lease_expires_at_utc: new Date(now + descriptor.ttlMs).toISOString(),
    content: descriptor.initialContent,
    workstation_handoff: {
      session_id: surfaceSessionId,
      mode: 'same_live_session',
    },
    created_at_utc: existing?.created_at_utc ?? nowUtc,
    updated_at_utc: nowUtc,
  }
  const next = {
    ...registry,
    updated_at_utc: nowUtc,
    sessions: { ...registry.sessions, [descriptor.slotId]: session },
  }
  return {
    ok: true,
    registry: next,
    session,
    message: 'claimed',
  }
}

export function refreshMonitorSessionLease(
  registry: MonitorSessionRegistryDescriptor,
  surfaceSessionId: string,
  owner: string,
  ttlMs: number,
  nowUtc = new Date().toISOString(),
): MonitorSessionClaimResult {
  const slotId = Object.entries(registry.sessions).find(
    ([, session]) => session.surface_session_id === surfaceSessionId,
  )?.[0]
  if (!slotId) {
    return {
      ok: false,
      registry,
      session: null,
      message: `Session '${surfaceSessionId}' not found`,
    }
  }
  const existing = registry.sessions[slotId]
  if (existing.owner !== owner) {
    return {
      ok: false,
      registry,
      session: existing,
      message: `Session '${surfaceSessionId}' is owned by '${existing.owner}'`,
    }
  }
  const next = {
    ...registry,
    updated_at_utc: nowUtc,
    sessions: {
      ...registry.sessions,
      [slotId]: {
        ...existing,
        revision: existing.revision + 1,
        lease_expires_at_utc: new Date(new Date(nowUtc).getTime() + ttlMs).toISOString(),
        updated_at_utc: nowUtc,
      },
    },
  }
  return {
    ok: true,
    registry: next,
    session: next.sessions[slotId],
    message: 'refreshed',
  }
}

export function releaseMonitorSession(
  registry: MonitorSessionRegistryDescriptor,
  surfaceSessionId: string,
  owner: string,
  nowUtc = new Date().toISOString(),
): MonitorSessionClaimResult {
  const slotId = Object.entries(registry.sessions).find(
    ([, session]) => session.surface_session_id === surfaceSessionId,
  )?.[0]
  if (!slotId) {
    return {
      ok: false,
      registry,
      session: null,
      message: `Session '${surfaceSessionId}' not found`,
    }
  }
  const existing = registry.sessions[slotId]
  if (existing.owner !== owner) {
    return {
      ok: false,
      registry,
      session: existing,
      message: `Session '${surfaceSessionId}' is owned by '${existing.owner}'`,
    }
  }
  const next = {
    ...registry,
    updated_at_utc: nowUtc,
    sessions: { ...registry.sessions, [slotId]: undefined },
  }
  delete next.sessions[slotId]
  return {
    ok: true,
    registry: next,
    session: null,
    message: 'released',
  }
}

export function serializeOwner(owner: AgentSurfaceOwner): string {
  switch (owner.kind) {
    case 'agent':
      return `agent:${owner.name}`
    case 'operator':
      return `operator:${owner.id}`
    case 'system':
      return `system:${owner.name}`
  }
}

export function isMonitorSessionRecord(value: unknown): value is MonitorSurfaceSessionRecord {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const record = value as Record<string, unknown>
  return (
    typeof record.slot_id === 'string' &&
    typeof record.session_id === 'string' &&
    typeof record.surface_session_id === 'string' &&
    typeof record.owner === 'string' &&
    typeof record.kind === 'string' &&
    typeof record.revision === 'number' &&
    typeof record.opened_at_utc === 'string' &&
    typeof record.lease_expires_at_utc === 'string' &&
    typeof record.content === 'object' &&
    isMonitorPlaybackState(record.playback) &&
    typeof record.workstation_handoff === 'object' &&
    typeof record.created_at_utc === 'string' &&
    typeof record.updated_at_utc === 'string'
  )
}

function isMonitorPlaybackState(value: unknown): boolean {
  if (value === undefined) return true
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const playback = value as Record<string, unknown>
  if (typeof playback.playing !== 'boolean') return false
  return ['currentTime', 'duration', 'volume'].every((field) => (
    playback[field] === undefined || (
      typeof playback[field] === 'number' &&
      Number.isFinite(playback[field]) &&
      playback[field] >= 0 &&
      (field !== 'volume' || playback[field] <= 1)
    )
  ))
}

export function isSessionActive(record: MonitorSurfaceSessionRecord, nowUtc = Date.now()): boolean {
  return new Date(record.lease_expires_at_utc).getTime() > nowUtc
}

export function registrySnapshot(registry: MonitorSessionRegistryDescriptor): string {
  return JSON.stringify(registry)
}

export function restoreRegistryFromSnapshot(
  snapshot: string,
): MonitorSessionRegistryDescriptor | null {
  return parseMonitorSessionRegistry(JSON.parse(snapshot))
}

export function toMonitorSurfaceSession(
  record: MonitorSurfaceSessionRecord,
): MonitorSurfaceSession {
  return {
    schemaVersion: MONITOR_SURFACE_SCHEMA_VERSION,
    surfaceSessionId: record.surface_session_id,
    slotId: record.slot_id,
    owner: record.owner,
    content: record.content,
    revision: record.revision,
    leaseExpiresAtUtc: record.lease_expires_at_utc,
    createdAtUtc: record.created_at_utc,
    updatedAtUtc: record.updated_at_utc,
    playback: record.playback,
  }
}
