// sigil: REPAIR
export const ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH = 'core/state/arda_boardroom_session_registry.json'
export const ARDA_BOARDROOM_SESSION_STORAGE_KEY = 'arda.boardroom.session_registry.v1'

export const BOARDROOM_SESSION_KINDS = ['monitor', 'desk', 'external', 'inline'] as const
export type BoardroomSessionKind = typeof BOARDROOM_SESSION_KINDS[number]

export interface BoardroomSessionRecord {
  slot_id: string
  kind: BoardroomSessionKind
  owner: string
  opened_at_utc: string
  lease_expires_at_utc: string
  metadata: Record<string, unknown>
}

export interface BoardroomSessionRegistry {
  schema_version: 'arda.boardroom.session_registry.v1'
  authority: typeof ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH
  updated_at_utc: string
  sessions: Record<string, BoardroomSessionRecord>
}

export interface BoardroomSessionClaim {
  ok: boolean
  registry: BoardroomSessionRegistry
  session: BoardroomSessionRecord | null
  message: string
}

export function createEmptyBoardroomSessionRegistry(updatedAtUtc = new Date(0).toISOString()): BoardroomSessionRegistry {
  return {
    schema_version: 'arda.boardroom.session_registry.v1',
    authority: ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH,
    updated_at_utc: updatedAtUtc,
    sessions: {},
  }
}

function isBoardroomSessionKind(value: unknown): value is BoardroomSessionKind {
  return typeof value === 'string' && (BOARDROOM_SESSION_KINDS as readonly string[]).includes(value)
}

function isBoardroomSessionRecord(value: unknown): value is BoardroomSessionRecord {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const record = value as Record<string, unknown>
  return (
    typeof record.slot_id === 'string' && record.slot_id.trim().length > 0 &&
    isBoardroomSessionKind(record.kind) &&
    typeof record.owner === 'string' && record.owner.trim().length > 0 &&
    typeof record.opened_at_utc === 'string' &&
    typeof record.lease_expires_at_utc === 'string' &&
    record.metadata && typeof record.metadata === 'object' && !Array.isArray(record.metadata)
  )
}

export function parseBoardroomSessionRegistry(value: unknown): BoardroomSessionRegistry | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const record = value as Record<string, unknown>
  if (record.schema_version !== 'arda.boardroom.session_registry.v1') return null
  const sessions: Record<string, BoardroomSessionRecord> = {}
  const rawSessions = record.sessions && typeof record.sessions === 'object' && !Array.isArray(record.sessions)
    ? (record.sessions as Record<string, unknown>)
    : {}
  for (const [slotId, raw] of Object.entries(rawSessions)) {
    if (!isBoardroomSessionRecord(raw)) continue
    sessions[slotId] = raw
  }
  return {
    schema_version: 'arda.boardroom.session_registry.v1',
    authority: ARDA_BOARDROOM_SESSION_REGISTRY_RELATIVE_PATH,
    updated_at_utc: typeof record.updated_at_utc === 'string' ? record.updated_at_utc : new Date(0).toISOString(),
    sessions,
  }
}

export function claimBoardroomSession(
  registry: BoardroomSessionRegistry,
  slotId: string,
  kind: BoardroomSessionKind,
  owner: string,
  ttlMs: number,
  metadata: Record<string, unknown> = {},
  nowUtc: string = new Date().toISOString(),
): BoardroomSessionClaim {
  if (ttlMs <= 0) {
    return { ok: false, registry, session: null, message: 'Session lease must be greater than 0ms' }
  }
  const existing = registry.sessions[slotId]
  const now = new Date(nowUtc).getTime()
  const existingExpiry = existing ? new Date(existing.lease_expires_at_utc).getTime() : 0
  const existingActive = existingExpiry > now
  if (existingActive && existing.owner !== owner) {
    return {
      ok: false,
      registry,
      session: existing,
      message: `Session for '${slotId}' is already owned by '${existing.owner}' until ${existing.lease_expires_at_utc}`,
    }
  }
  const session: BoardroomSessionRecord = {
    slot_id: slotId,
    kind,
    owner,
    opened_at_utc: existing?.opened_at_utc ?? nowUtc,
    lease_expires_at_utc: new Date(now + ttlMs).toISOString(),
    metadata,
  }
  const next = {
    ...registry,
    updated_at_utc: nowUtc,
    sessions: { ...registry.sessions, [slotId]: session },
  }
  return { ok: true, registry: next, session, message: existingActive ? `Refreshed session for '${slotId}'` : `Claimed session for '${slotId}'` }
}

export function releaseBoardroomSession(
  registry: BoardroomSessionRegistry,
  slotId: string,
  owner: string,
  nowUtc: string = new Date().toISOString(),
): BoardroomSessionRegistry {
  const existing = registry.sessions[slotId]
  if (!existing || existing.owner !== owner) return registry
  const next = { ...registry, updated_at_utc: nowUtc, sessions: { ...registry.sessions } }
  delete next.sessions[slotId]
  return next
}

export function refreshBoardroomSession(
  registry: BoardroomSessionRegistry,
  slotId: string,
  owner: string,
  ttlMs: number,
  nowUtc: string = new Date().toISOString(),
): BoardroomSessionClaim {
  if (ttlMs <= 0) {
    return { ok: false, registry, session: null, message: 'Session lease must be greater than 0ms' }
  }
  const existing = registry.sessions[slotId]
  if (!existing || existing.owner !== owner) {
    return { ok: false, registry, session: existing ?? null, message: `Agent '${owner}' does not own session '${slotId}'` }
  }
  const refreshed: BoardroomSessionRecord = {
    ...existing,
    lease_expires_at_utc: new Date(new Date(nowUtc).getTime() + ttlMs).toISOString(),
  }
  const next = {
    ...registry,
    updated_at_utc: nowUtc,
    sessions: { ...registry.sessions, [slotId]: refreshed },
  }
  return { ok: true, registry: next, session: refreshed, message: `Refreshed session for '${slotId}'` }
}

export function activeBoardroomSession(
  registry: BoardroomSessionRegistry,
  slotId: string,
  nowUtc: string = new Date().toISOString(),
): BoardroomSessionRecord | null {
  const session = registry.sessions[slotId]
  if (!session) return null
  return new Date(session.lease_expires_at_utc).getTime() > new Date(nowUtc).getTime() ? session : null
}
