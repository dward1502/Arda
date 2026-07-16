// sigil: REPAIR
import {
  type HudEventDomain,
  type HudEventMetrics,
  type HudEventRef,
  type HudEventSeverity,
  type HudEventTrace,
  type HudEventV1,
  normalizeHudEvent,
} from './hudEventSchema'

export const ARDA_PRESENCE_SCHEMA_VERSION = 'arda.presence.v1' as const

export type ArdaPresenceDomain =
  | 'agent'
  | 'mission'
  | 'knowledge'
  | 'operations'
  | 'governance'
  | 'security'
  | 'telemetry'
  | 'system'
  | 'external'

export type ArdaPresenceKind =
  | 'presence.agent_state'
  | 'presence.agent_message'
  | 'presence.file_event'
  | 'presence.system_event'
  | 'presence.consensus_state'
  | 'presence.conflict_state'

export type ArdaPresenceEntityType = 'agent' | 'human' | 'service' | 'model' | 'file' | 'mission' | 'system'

export interface ArdaPresenceEntity {
  id: string
  type: ArdaPresenceEntityType
  label: string
  role?: string
  status?: 'idle' | 'active' | 'thinking' | 'blocked' | 'error' | 'offline'
  parentId?: string
  childIds?: string[]
  currentTask?: string
  missionId?: string
  confidence?: number
}

export interface ArdaPresenceMessage {
  fromEntityId: string
  toEntityIds: string[]
  channel: 'boardroom' | 'direct' | 'broadcast' | 'system' | 'external'
  summary: string
  bodyPreview?: string
  transport?: 'websocket' | 'sse' | 'unix_socket' | 'stdin' | 'file_tail' | 'http' | 'unknown'
  sentiment?: 'aligned' | 'neutral' | 'concern' | 'conflict'
}

export interface ArdaPresenceFileEvent {
  path: string
  operation: 'read' | 'created' | 'modified' | 'deleted' | 'indexed' | 'reviewed'
  canonicality?: 'raw' | 'candidate' | 'canonical' | 'rejected'
  sourceId?: string
}

export interface ArdaPresenceConsensus {
  subjectId: string
  state: 'forming' | 'aligned' | 'disputed' | 'blocked' | 'resolved'
  participants: string[]
  summary: string
  triadScore?: number
  resonanceScore?: number
}

export interface ArdaPresenceSceneBinding {
  scene: 'boardroom' | 'world'
  zoneId?: string
  anchorId?: string
  workstationId?: string
  slotId?: string
  visualHint?: 'avatar_pulse' | 'message_beam' | 'monitor_card' | 'workstation_feed' | 'conflict_marker'
}

export interface ArdaPresenceTrace {
  taskId?: string
  missionId?: string
  agentId?: string
  correlationId?: string
  sourcePath?: string
  sourceLine?: number
  transportOffset?: string
}

export interface ArdaPresenceEventV1 {
  schemaVersion: typeof ARDA_PRESENCE_SCHEMA_VERSION
  id: string
  timestamp: string | number | Date
  source: string
  domain: ArdaPresenceDomain
  kind: ArdaPresenceKind
  severity: HudEventSeverity
  entity?: ArdaPresenceEntity
  message?: ArdaPresenceMessage
  file?: ArdaPresenceFileEvent
  consensus?: ArdaPresenceConsensus
  scene?: ArdaPresenceSceneBinding
  trace?: ArdaPresenceTrace
  tags?: string[]
  raw?: Record<string, unknown>
}

export type ArdaPresenceEventInput = Partial<ArdaPresenceEventV1> & {
  id: string | number
  timestamp?: string | number | Date
  source?: string
  domain?: ArdaPresenceDomain
  kind?: ArdaPresenceKind
  severity?: HudEventSeverity
}

function compactStrings(values: Array<string | null | undefined>): string[] {
  return values.flatMap((value) => {
    if (typeof value !== 'string') return []
    const trimmed = value.trim()
    return trimmed.length > 0 ? [trimmed] : []
  })
}

function uniqueStrings(values: string[]): string[] {
  return Array.from(new Set(values))
}

function addRef(refs: HudEventRef[], ref: HudEventRef | null) {
  if (!ref) return
  const key = `${ref.type}:${ref.id}`
  if (refs.some((existing) => `${existing.type}:${existing.id}` === key)) return
  refs.push(ref)
}

function entityRef(entity: ArdaPresenceEntity | undefined): HudEventRef | null {
  if (!entity?.id) return null
  if (entity.type === 'agent') return { type: 'agent', id: entity.id }
  if (entity.type === 'mission') return { type: 'mission', id: entity.id }
  if (entity.type === 'service' || entity.type === 'system' || entity.type === 'model') return { type: 'node', id: entity.id }
  return null
}

function validDomain(value: string | undefined): ArdaPresenceDomain {
  const accepted: ArdaPresenceDomain[] = [
    'agent',
    'mission',
    'knowledge',
    'operations',
    'governance',
    'security',
    'telemetry',
    'system',
    'external',
  ]
  return accepted.includes(value as ArdaPresenceDomain) ? (value as ArdaPresenceDomain) : 'system'
}

function validKind(value: string | undefined): ArdaPresenceKind {
  const accepted: ArdaPresenceKind[] = [
    'presence.agent_state',
    'presence.agent_message',
    'presence.file_event',
    'presence.system_event',
    'presence.consensus_state',
    'presence.conflict_state',
  ]
  return accepted.includes(value as ArdaPresenceKind) ? (value as ArdaPresenceKind) : 'presence.system_event'
}

function validSeverity(value: string | undefined): HudEventSeverity {
  const accepted: HudEventSeverity[] = ['debug', 'info', 'warn', 'error', 'critical']
  return accepted.includes(value as HudEventSeverity) ? (value as HudEventSeverity) : 'info'
}

function defaultMessage(event: ArdaPresenceEventV1): string {
  if (event.message?.summary) return event.message.summary
  if (event.consensus?.summary) return event.consensus.summary
  if (event.file?.operation && event.file.path) return `${event.file.operation} ${event.file.path}`
  if (event.entity?.label && event.entity.status) return `${event.entity.label} is ${event.entity.status}`
  if (event.entity?.label) return `${event.entity.label} emitted ${event.kind}`
  return `${event.source} emitted ${event.kind}`
}

function buildMetrics(event: ArdaPresenceEventV1): HudEventMetrics | undefined {
  const metrics: HudEventMetrics = {}
  if (Number.isFinite(event.consensus?.triadScore)) metrics.triadScore = event.consensus?.triadScore
  return Object.keys(metrics).length > 0 ? metrics : undefined
}

function buildTrace(event: ArdaPresenceEventV1): HudEventTrace | undefined {
  const trace: HudEventTrace = {}
  if (event.trace?.taskId) trace.taskId = event.trace.taskId
  if (event.trace?.missionId) trace.missionId = event.trace.missionId
  if (event.trace?.agentId) trace.agentId = event.trace.agentId
  if (event.trace?.correlationId) trace.correlationId = event.trace.correlationId
  return Object.keys(trace).length > 0 ? trace : undefined
}

export function normalizeArdaPresenceEvent(event: ArdaPresenceEventInput): ArdaPresenceEventV1 {
  return {
    schemaVersion: ARDA_PRESENCE_SCHEMA_VERSION,
    id: String(event.id),
    timestamp: event.timestamp ?? new Date(),
    source: String(event.source || 'arda-presence'),
    domain: validDomain(event.domain),
    kind: validKind(event.kind),
    severity: validSeverity(event.severity),
    entity: event.entity,
    message: event.message,
    file: event.file,
    consensus: event.consensus,
    scene: event.scene,
    trace: event.trace,
    tags: Array.isArray(event.tags) ? uniqueStrings(compactStrings(event.tags)) : undefined,
    raw: event.raw,
  }
}

export function presenceRefs(event: ArdaPresenceEventV1): HudEventRef[] {
  const refs: HudEventRef[] = []
  addRef(refs, entityRef(event.entity))
  if (event.entity?.missionId) addRef(refs, { type: 'mission', id: event.entity.missionId })
  if (event.trace?.agentId) addRef(refs, { type: 'agent', id: event.trace.agentId })
  if (event.trace?.missionId) addRef(refs, { type: 'mission', id: event.trace.missionId })
  return refs
}

export function presenceTags(event: ArdaPresenceEventV1): string[] {
  return uniqueStrings(compactStrings([
    ...(event.tags ?? []),
    `presence:${event.kind.replace(/^presence\./, '')}`,
    event.message?.channel ? `channel:${event.message.channel}` : undefined,
    event.message?.transport ? `transport:${event.message.transport}` : undefined,
    event.message?.sentiment ? `sentiment:${event.message.sentiment}` : undefined,
    event.file?.operation ? `file:${event.file.operation}` : undefined,
    event.file?.canonicality ? `canonicality:${event.file.canonicality}` : undefined,
    event.consensus?.state ? `consensus:${event.consensus.state}` : undefined,
    event.scene?.scene ? `scene:${event.scene.scene}` : undefined,
    event.scene?.zoneId ? `zone:${event.scene.zoneId}` : undefined,
    event.scene?.anchorId ? `anchor:${event.scene.anchorId}` : undefined,
    event.scene?.workstationId ? `workstation:${event.scene.workstationId}` : undefined,
    event.scene?.slotId ? `slot:${event.scene.slotId}` : undefined,
    event.scene?.visualHint ? `visual:${event.scene.visualHint}` : undefined,
    event.entity?.type ? `entity:${event.entity.type}` : undefined,
    event.entity?.status ? `status:${event.entity.status}` : undefined,
  ]))
}

export function presenceMessage(event: ArdaPresenceEventV1): string {
  return defaultMessage(event)
}

export function projectArdaPresenceToHudEvent(event: ArdaPresenceEventInput): HudEventV1 {
  const normalized = normalizeArdaPresenceEvent(event)
  return normalizeHudEvent({
    id: normalized.id,
    timestamp: normalized.timestamp,
    severity: normalized.severity,
    source: normalized.source,
    message: presenceMessage(normalized),
    refs: presenceRefs(normalized),
    pinned: normalized.severity === 'error' || normalized.severity === 'critical',
    domain: normalized.domain as HudEventDomain,
    kind: normalized.kind,
    tags: presenceTags(normalized),
    metrics: buildMetrics(normalized),
    trace: buildTrace(normalized),
    raw: {
      presence: normalized,
      adapter: {
        schemaVersion: ARDA_PRESENCE_SCHEMA_VERSION,
        projectedAt: new Date().toISOString(),
      },
    },
  })
}
