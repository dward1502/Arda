// sigil: REPAIR
import type { SystemActionExecutionEvent } from './systemActionBus'
import { resolveSourceSigilsFromContract, stateSignalFromContract } from './soterionRender'

export const HUD_EVENT_SCHEMA_VERSION = '1.0.0' as const

export type HudEventSeverity = 'debug' | 'info' | 'warn' | 'error' | 'critical'
export type HudEventRefType = 'agent' | 'mission' | 'node'
export type HudEventDomain =
  | 'system'
  | 'operations'
  | 'agent'
  | 'mission'
  | 'inventory'
  | 'dock'
  | 'governance'
  | 'security'
  | 'telemetry'
  | 'knowledge'
  | 'external'
  | 'unknown'

export interface HudEventRef {
  type: HudEventRefType
  id: string
}

export interface HudEventMetrics {
  jwEstimated?: number
  leScore?: number
  triadScore?: number
  durationMs?: number
}

export interface HudEventTrace {
  taskId?: string
  missionId?: string
  agentId?: string
  correlationId?: string
}

export interface HudEventV1 {
  id: string
  timestamp: Date
  severity: HudEventSeverity
  source: string
  message: string
  refs: HudEventRef[]
  pinned: boolean
  schemaVersion: typeof HUD_EVENT_SCHEMA_VERSION
  kind: string
  domain: HudEventDomain
  sigils?: string[]
  tags?: string[]
  metrics?: HudEventMetrics
  trace?: HudEventTrace
  raw?: Record<string, unknown>
}

type LegacyHudEvent = Omit<Partial<HudEventV1>, 'id' | 'timestamp'> & {
  id?: string | number
  timestamp?: Date | string | number
}

function defaultDomainFromSource(source: string): HudEventDomain {
  const normalized = source.toLowerCase()
  if (normalized.includes('inventory')) return 'inventory'
  if (normalized.includes('dock')) return 'dock'
  if (normalized.includes('agent')) return 'agent'
  if (normalized.includes('mission')) return 'mission'
  if (normalized.includes('triad') || normalized.includes('governance')) return 'governance'
  if (normalized.includes('warden') || normalized.includes('hades') || normalized.includes('security')) return 'security'
  if (normalized.includes('joule') || normalized.includes('telemetry')) return 'telemetry'
  if (normalized.includes('athena') || normalized.includes('knowledge')) return 'knowledge'
  if (normalized.includes('system')) return 'system'
  return 'unknown'
}

function normalizeTimestamp(value: LegacyHudEvent['timestamp']): Date {
  if (value instanceof Date) return value
  if (typeof value === 'number') return new Date(value)
  if (typeof value === 'string') {
    const parsed = new Date(value)
    if (!Number.isNaN(parsed.getTime())) return parsed
  }
  return new Date()
}

function generateEventId(source: string): string {
  const safeSource = source.toLowerCase().replace(/[^a-z0-9_-]/g, '-').slice(0, 24) || 'event'
  const rand = Math.random().toString(36).slice(2, 8)
  return `${safeSource}-${Date.now()}-${rand}`
}

export function normalizeHudEvent(event: LegacyHudEvent): HudEventV1 {
  const source = String(event.source || 'system')
  const domain = event.domain || defaultDomainFromSource(source)
  const defaultSigils = resolveSourceSigilsFromContract(null, source)
  return {
    id: event.id !== undefined ? String(event.id) : generateEventId(source),
    timestamp: normalizeTimestamp(event.timestamp),
    severity: event.severity || 'info',
    source,
    message: String(event.message || 'Event emitted'),
    refs: Array.isArray(event.refs) ? event.refs : [],
    pinned: Boolean(event.pinned),
    schemaVersion: HUD_EVENT_SCHEMA_VERSION,
    kind: event.kind || `${domain}.event`,
    domain,
    sigils: Array.isArray(event.sigils) ? event.sigils : defaultSigils.length ? defaultSigils : undefined,
    tags: Array.isArray(event.tags) ? event.tags : undefined,
    metrics: event.metrics,
    trace: event.trace,
    raw: event.raw,
  }
}

export function buildSystemActionHudEvent(event: SystemActionExecutionEvent): HudEventV1 {
  const severity: HudEventSeverity = event.result.ok ? 'info' : 'error'
  const sigils = resolveSourceSigilsFromContract(null, `action:${event.context.source}`)
  const failureSignal = stateSignalFromContract(null, 'ESCALATION')
  if (!event.result.ok && failureSignal) sigils.push(failureSignal)
  return normalizeHudEvent({
    id: `action-${event.action}-${event.at}`,
    timestamp: event.at,
    severity,
    source: `action:${event.context.source}`,
    message: event.result.ok
      ? `Action ${event.action} completed via ${event.result.provider}`
      : `Action ${event.action} failed via ${event.result.provider}: ${event.result.message}`,
    refs: [],
    pinned: !event.result.ok,
    domain: 'operations',
    kind: 'operations.system_action',
    sigils,
    tags: [event.action, event.context.persona, event.context.mood, event.result.provider],
    metrics: {
      durationMs: event.durationMs,
    },
    trace: {
      correlationId: `${event.action}:${event.at}`,
    },
    raw: {
      context: event.context as unknown as Record<string, unknown>,
      result: event.result as unknown as Record<string, unknown>,
    },
  })
}
