import type { BoardroomAgentClaim } from '../../lib/boardroomSlotSettings'
import type { MonitorSurfaceSessionRecord } from '../../lib/monitorSurfaceContract'

export type MonitorOwnershipSource = 'session' | 'claim' | 'idle'
export type MonitorLeaseState = 'healthy' | 'expiring' | 'expired' | 'idle'

export interface MonitorOwnershipRailModel {
  occupied: boolean
  source: MonitorOwnershipSource
  owner: string | null
  contentKind: string | null
  color: string
  fingerprint: boolean[]
  leaseState: MonitorLeaseState
  leaseRemainingMs: number | null
}

function hashOwner(owner: string): number {
  let hash = 0x811c9dc5
  for (let index = 0; index < owner.length; index += 1) {
    hash ^= owner.charCodeAt(index)
    hash = Math.imul(hash, 0x01000193)
  }
  return hash >>> 0
}

function ownerColor(owner: string): string {
  const hash = hashOwner(owner)
  const hue = (hash % 300) + 20
  return `hsl(${hue} 88% 68%)`
}

function ownerFingerprint(owner: string): boolean[] {
  let state = hashOwner(owner) || 1
  return Array.from({ length: 12 }, (_, index) => {
    state ^= state << 13
    state ^= state >>> 17
    state ^= state << 5
    return ((state >>> (index % 16)) & 1) === 1
  })
}

function resolveLeaseState(expiresAtUtc: string, nowUtc: string): Pick<MonitorOwnershipRailModel, 'leaseState' | 'leaseRemainingMs'> {
  const expiry = Date.parse(expiresAtUtc)
  const now = Date.parse(nowUtc)
  if (!Number.isFinite(expiry) || !Number.isFinite(now)) return { leaseState: 'expired', leaseRemainingMs: 0 }
  const leaseRemainingMs = expiry - now
  if (leaseRemainingMs <= 0) return { leaseState: 'expired', leaseRemainingMs }
  if (leaseRemainingMs <= 60_000) return { leaseState: 'expiring', leaseRemainingMs }
  return { leaseState: 'healthy', leaseRemainingMs }
}

export function deriveMonitorOwnershipRail(
  session: MonitorSurfaceSessionRecord | null,
  claim: BoardroomAgentClaim | null,
  nowUtc: string,
): MonitorOwnershipRailModel {
  if (!session && !claim) {
    return {
      occupied: false,
      source: 'idle',
      owner: null,
      contentKind: null,
      color: '#27404d',
      fingerprint: Array.from({ length: 12 }, () => false),
      leaseState: 'idle',
      leaseRemainingMs: null,
    }
  }

  const owner = session?.owner ?? claim?.owner ?? 'unknown'
  const expiresAtUtc = session?.lease_expires_at_utc ?? claim?.lease_expires_at_utc ?? nowUtc
  return {
    occupied: true,
    source: session ? 'session' : 'claim',
    owner,
    contentKind: session?.content.kind ?? claim?.activity_kind ?? null,
    color: ownerColor(owner),
    fingerprint: ownerFingerprint(owner),
    ...resolveLeaseState(expiresAtUtc, nowUtc),
  }
}
