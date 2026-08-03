import {
  BOARDROOM_MONITOR_SLOT_IDS,
  type BoardroomAgentClaim,
  type BoardroomSurfaceLayout,
  type MonitorSurfaceRequest,
} from '../../lib/boardroomSlotSettings'

export interface MonitorSurfacePayloadEvent {
  slotId: string
  payloadBinding: string
  content: string
  mime: string
}

export interface MonitorClaimChangedEvent {
  slotId: string
  owner: string
  activityKind: BoardroomAgentClaim['activity_kind']
  payloadBinding: string
  focusMode: string
  leaseExpiresAtUtc: string
  active: boolean
}

export function formatMonitorSurfaceStream(
  payload: Pick<MonitorSurfacePayloadEvent, 'content' | 'mime'> | null,
  reducedMotion: boolean,
): string {
  if (reducedMotion) return '[NO DATA — reduced-motion mode active]'
  const content = payload?.content.trim()
  return content ? content : '[NO DATA — awaiting agent payload]'
}

export function normalizeMonitorLeaseExpiry(value: string): string {
  if (value.startsWith('unix:')) {
    const seconds = Number(value.slice('unix:'.length))
    if (Number.isFinite(seconds)) return new Date(seconds * 1000).toISOString()
  }
  const timestamp = new Date(value)
  return Number.isNaN(timestamp.getTime()) ? '' : timestamp.toISOString()
}

export function claimFromMonitorEvent(
  event: MonitorClaimChangedEvent,
  fallbackPreview: BoardroomSurfaceLayout['preview'],
): BoardroomAgentClaim {
  return {
    owner: event.owner,
    activity_kind: event.activityKind,
    payload_binding: event.payloadBinding,
    fallback_preview: fallbackPreview,
    lease_expires_at_utc: normalizeMonitorLeaseExpiry(event.leaseExpiresAtUtc),
  }
}

export function resolveMonitorContractSlotId(sceneZoneId: string, assignmentSlotId: string | undefined): string {
  return assignmentSlotId ?? sceneZoneId
}

export function resolveMonitorSurfaceOpenRequest(
  slotId: string,
  sourceZoneId: string | null,
  focusMode: string,
): MonitorSurfaceRequest | null {
  if (!BOARDROOM_MONITOR_SLOT_IDS.includes(slotId as typeof BOARDROOM_MONITOR_SLOT_IDS[number])) return null
  if (!sourceZoneId) return null
  return {
    slotId,
    sourceZoneId: sourceZoneId.startsWith('hermes.') ? 'hermes_runtime' : sourceZoneId,
    focusMode,
    title: `ARDA Monitor — ${slotId}`,
  }
}
