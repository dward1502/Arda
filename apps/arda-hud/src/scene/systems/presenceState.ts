// sigil: REPAIR
import {
  type ArdaPresenceDomain,
  type ArdaPresenceEventInput,
  type ArdaPresenceEventV1,
  normalizeArdaPresenceEvent,
} from '../../lib/ardaPresenceSchema'
import { parseJsonOrNull } from '../../lib/jsonParse'
import type {
  AgentPresenceState,
  PresenceLedgerFreshness,
  PresenceLedgerProjection,
  PresenceLedgerStatus,
  PresencePhase,
  PresenceScenario,
  PresenceSource,
  PresenceSupportMarker,
  PresenceUrgency,
  PresenceVisualState,
  SovereignAgentId,
} from './presenceTypes'

export const DEFAULT_AGENT_PRESENCE_STATE: AgentPresenceState = Object.freeze({
  scenario: 'idle',
  phase: 'idle',
  primaryAgent: 'arandur',
  supportAgents: Object.freeze([]) as SovereignAgentId[],
  focus: Object.freeze([]) as SovereignAgentId[],
  urgency: 'normal',
  source: 'arda',
  timestamp: '1970-01-01T00:00:00.000Z',
})

const PRESENCE_SCENARIOS: PresenceScenario[] = ['idle', 'briefing', 'routing', 'knowledge', 'council', 'alert', 'recovery']
const PRESENCE_PHASES: PresencePhase[] = ['idle', 'agent_arrival', 'alert', 'awaiting_user', 'action_confirmed', 'resolved']
const PRESENCE_URGENCIES: PresenceUrgency[] = ['normal', 'high']
const PRESENCE_SOURCES: PresenceSource[] = ['arda', 'arda', 'operator', 'citadel']
const SOVEREIGN_AGENT_IDS: SovereignAgentId[] = [
  'arandur',
  'athena',
  'hermes',
  'manwe',
  'warden',
  'oracle',
  'plutus',
  'apollo',
  'mnemosyne',
  'prometheus',
  'hades',
]

function isPresenceScenario(value: unknown): value is PresenceScenario {
  return PRESENCE_SCENARIOS.includes(value as PresenceScenario)
}

function isPresencePhase(value: unknown): value is PresencePhase {
  return PRESENCE_PHASES.includes(value as PresencePhase)
}

function isPresenceUrgency(value: unknown): value is PresenceUrgency {
  return PRESENCE_URGENCIES.includes(value as PresenceUrgency)
}

function isPresenceSource(value: unknown): value is PresenceSource {
  return PRESENCE_SOURCES.includes(value as PresenceSource)
}

function isSovereignAgentId(value: unknown): value is SovereignAgentId {
  return SOVEREIGN_AGENT_IDS.includes(value as SovereignAgentId)
}

function uniqueAgents(values: unknown, exclude: SovereignAgentId[] = []): SovereignAgentId[] {
  if (!Array.isArray(values)) return []
  const excluded = new Set(exclude)
  const agents: SovereignAgentId[] = []
  values.forEach((value) => {
    if (!isSovereignAgentId(value)) return
    if (excluded.has(value)) return
    if (agents.includes(value)) return
    agents.push(value)
  })
  return agents
}

function cleanOptionalText(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed.slice(0, 180) : undefined
}

function normalizeTimestamp(value: unknown, fallbackTimestamp: string): string {
  if (typeof value === 'string' && Number.isFinite(Date.parse(value))) return new Date(value).toISOString()
  if (value instanceof Date && Number.isFinite(value.getTime())) return value.toISOString()
  if (typeof value === 'number' && Number.isFinite(value)) return new Date(value).toISOString()
  return fallbackTimestamp
}

export function normalizeAgentPresenceState(
  input: Partial<AgentPresenceState> | null | undefined,
  fallbackTimestamp = new Date().toISOString(),
): AgentPresenceState {
  const primaryAgent = isSovereignAgentId(input?.primaryAgent) ? input.primaryAgent : DEFAULT_AGENT_PRESENCE_STATE.primaryAgent
  const supportAgents = uniqueAgents(input?.supportAgents, [primaryAgent])
  const focus = uniqueAgents(input?.focus)
  return {
    scenario: isPresenceScenario(input?.scenario) ? input.scenario : DEFAULT_AGENT_PRESENCE_STATE.scenario,
    phase: isPresencePhase(input?.phase) ? input.phase : DEFAULT_AGENT_PRESENCE_STATE.phase,
    primaryAgent,
    supportAgents,
    focus,
    urgency: isPresenceUrgency(input?.urgency) ? input.urgency : DEFAULT_AGENT_PRESENCE_STATE.urgency,
    ...Object.fromEntries(
      [
        ['banner', cleanOptionalText(input?.banner)],
        ['inquiry', cleanOptionalText(input?.inquiry)],
        ['action', cleanOptionalText(input?.action)],
        ['response', cleanOptionalText(input?.response)],
        ['eventId', cleanOptionalText(input?.eventId)],
        ['missionId', cleanOptionalText(input?.missionId)],
        ['anchorTarget', cleanOptionalText(input?.anchorTarget)],
      ].filter(([, value]) => value !== undefined),
    ),
    source: isPresenceSource(input?.source) ? input.source : DEFAULT_AGENT_PRESENCE_STATE.source,
    timestamp: normalizeTimestamp(input?.timestamp, fallbackTimestamp),
  }
}

function scenarioForDomain(domain: ArdaPresenceDomain, severity: ArdaPresenceEventV1['severity']): PresenceScenario {
  if (severity === 'error' || severity === 'critical' || severity === 'warn') return 'alert'
  if (domain === 'knowledge') return 'knowledge'
  if (domain === 'governance') return 'council'
  if (domain === 'operations' || domain === 'mission') return 'routing'
  if (domain === 'security') return 'alert'
  return 'briefing'
}

function phaseForPresence(event: ArdaPresenceEventV1): PresencePhase {
  if (event.severity === 'error' || event.severity === 'critical') return 'alert'
  if (event.severity === 'warn' || event.message?.sentiment === 'concern' || event.consensus?.state === 'forming') return 'awaiting_user'
  if (event.entity?.status === 'active' || event.entity?.status === 'thinking') return 'agent_arrival'
  if (event.consensus?.state === 'resolved') return 'resolved'
  return 'agent_arrival'
}

function urgencyForPresence(event: ArdaPresenceEventV1): PresenceUrgency {
  return event.severity === 'warn' || event.severity === 'error' || event.severity === 'critical' ? 'high' : 'normal'
}

function sourceForPresence(source: string): PresenceSource {
  return isPresenceSource(source) ? source : 'arda'
}

export function derivePresenceStateFromArdaPresence(eventInput: ArdaPresenceEventInput): AgentPresenceState {
  const event = normalizeArdaPresenceEvent(eventInput)
  const primaryAgent = isSovereignAgentId(event.entity?.id)
    ? event.entity.id
    : isSovereignAgentId(event.trace?.agentId)
      ? event.trace.agentId
      : DEFAULT_AGENT_PRESENCE_STATE.primaryAgent
  const addressedAgents = uniqueAgents(event.message?.toEntityIds, [primaryAgent])
  const focus = uniqueAgents(event.message?.toEntityIds)
  return normalizeAgentPresenceState({
    scenario: scenarioForDomain(event.domain, event.severity),
    phase: phaseForPresence(event),
    primaryAgent,
    supportAgents: addressedAgents.filter((agent) => agent !== 'arandur'),
    focus,
    urgency: urgencyForPresence(event),
    banner: event.message?.summary,
    eventId: event.id,
    source: sourceForPresence(event.source),
    timestamp: normalizeTimestamp(event.timestamp, new Date().toISOString()),
  })
}

export interface ArdaPresenceEventLedgerRecord {
  id?: unknown
  schema?: unknown
  kind?: unknown
  domain?: unknown
  timestamp_utc?: unknown
  entity?: {
    agent?: unknown
    mission_id?: unknown
  }
  scene?: {
    presence?: {
      attention?: unknown
      mode?: unknown
      accent?: unknown
      anchor_target?: unknown
      support_agents?: unknown
      focus_agents?: unknown
    }
  }
  trace?: {
    correlation_id?: unknown
  }
}

function readObject(value: unknown): Record<string, unknown> | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? value as Record<string, unknown> : null
}

function readString(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : undefined
}

function readSovereignAgentList(value: unknown): SovereignAgentId[] {
  if (!Array.isArray(value)) return []
  return value
    .map((agent) => readString(agent))
    .filter((agent): agent is SovereignAgentId => isSovereignAgentId(agent))
}

function presenceScenarioForLedger(domain: string | undefined, mode: string | undefined): PresenceScenario {
  if (domain === 'knowledge') return 'knowledge'
  return presenceScenarioForMode(mode)
}

function presenceScenarioForMode(mode: string | undefined): PresenceScenario {
  if (mode === 'coordinating' || mode === 'executing') return 'routing'
  if (mode === 'escalating') return 'alert'
  if (mode === 'advising') return 'briefing'
  if (mode === 'offline') return 'recovery'
  return 'idle'
}

function presencePhaseForMode(mode: string | undefined, attention: string | undefined): PresencePhase {
  if (attention === 'critical') return 'alert'
  if (mode === 'escalating' || attention === 'elevated') return 'awaiting_user'
  if (mode === 'offline') return 'resolved'
  if (mode === 'observing') return 'idle'
  return 'agent_arrival'
}

function presenceUrgencyForAttention(attention: string | undefined): PresenceUrgency {
  return attention === 'elevated' || attention === 'critical' ? 'high' : 'normal'
}

function titleAgent(agent: SovereignAgentId): string {
  return agent.toUpperCase()
}

function derivePresenceStateFromEventLedgerRecord(record: ArdaPresenceEventLedgerRecord, fallbackTimestamp: string): AgentPresenceState | null {
  if (readString(record.schema) !== 'arda.arda.presence_event.v1') return null
  if (readString(record.kind) !== 'presence.agent_state') return null

  const entity = readObject(record.entity)
  const scene = readObject(record.scene)
  const presence = readObject(scene?.presence)
  const agentValue = readString(entity?.agent)
  const primaryAgent = isSovereignAgentId(agentValue) ? agentValue : DEFAULT_AGENT_PRESENCE_STATE.primaryAgent
  const mode = readString(presence?.mode)
  const attention = readString(presence?.attention)
  const anchorTarget = readString(presence?.anchor_target)
  const banner = mode && attention && anchorTarget
    ? `${titleAgent(primaryAgent)} ${mode} with ${attention} attention at ${anchorTarget}`
    : undefined
  const supportAgents = readSovereignAgentList(presence?.support_agents)
  const explicitFocus = readSovereignAgentList(presence?.focus_agents)
  const focus = explicitFocus.length > 0 ? uniqueAgents([primaryAgent, ...explicitFocus]) : [primaryAgent]

  return normalizeAgentPresenceState({
    scenario: presenceScenarioForLedger(readString(record.domain), mode),
    phase: presencePhaseForMode(mode, attention),
    primaryAgent,
    supportAgents,
    focus,
    urgency: presenceUrgencyForAttention(attention),
    banner,
    eventId: readString(record.id),
    missionId: readString(entity?.mission_id),
    anchorTarget,
    source: 'arda',
    timestamp: normalizeTimestamp(record.timestamp_utc, fallbackTimestamp),
  }, fallbackTimestamp)
}

const PRESENCE_LEDGER_FRESH_AFTER_MS = 15 * 60 * 1000

function presenceLedgerFreshness(latestTimestamp: string | undefined, fallbackTimestamp: string): PresenceLedgerFreshness {
  if (!latestTimestamp) return 'unknown'
  const latestMs = Date.parse(latestTimestamp)
  const fallbackMs = Date.parse(fallbackTimestamp)
  if (!Number.isFinite(latestMs) || !Number.isFinite(fallbackMs)) return 'unknown'
  return Math.max(0, fallbackMs - latestMs) <= PRESENCE_LEDGER_FRESH_AFTER_MS ? 'fresh' : 'stale'
}

function presenceLedgerAgeSeconds(latestTimestamp: string | undefined, fallbackTimestamp: string): number | undefined {
  if (!latestTimestamp) return undefined
  const latestMs = Date.parse(latestTimestamp)
  const fallbackMs = Date.parse(fallbackTimestamp)
  if (!Number.isFinite(latestMs) || !Number.isFinite(fallbackMs)) return undefined
  return Math.max(0, Math.floor((fallbackMs - latestMs) / 1000))
}

function presenceLedgerSummary(status: Omit<PresenceLedgerStatus, 'summary'>): string {
  if (status.source === 'fallback_default') {
    if (status.malformedLineCount > 0 || status.ignoredLineCount > 0) {
      return `Fallback default: no valid agent presence rows (${status.ignoredLineCount} ignored, ${status.malformedLineCount} malformed)`
    }
    return 'Fallback default: no presence ledger rows'
  }
  const eventLabel = status.latestEventId ? `event ${status.latestEventId}` : 'latest event'
  const ageLabel = typeof status.ageSeconds === 'number' ? `${status.ageSeconds}s old` : 'age unknown'
  return `Live presence ledger ${status.freshness}: ${eventLabel}, ${ageLabel}`
}

export function derivePresenceLedgerProjection(
  ledgerText: string,
  fallbackTimestamp = new Date().toISOString(),
): PresenceLedgerProjection {
  const fallback = normalizeAgentPresenceState(DEFAULT_AGENT_PRESENCE_STATE, fallbackTimestamp)
  const result = ledgerText
    .split(/\r?\n/)
    .reduce<{
      state: AgentPresenceState
      validEventCount: number
      ignoredLineCount: number
      malformedLineCount: number
    }>((latest, line) => {
      const trimmed = line.trim()
      if (!trimmed) return latest
      try {
        const parsed = parseJsonOrNull<ArdaPresenceEventLedgerRecord>(trimmed)
        if (!parsed) return { ...latest, malformedLineCount: latest.malformedLineCount + 1 }
        const state = derivePresenceStateFromEventLedgerRecord(parsed, fallbackTimestamp)
        if (!state) {
          return { ...latest, ignoredLineCount: latest.ignoredLineCount + 1 }
        }
        return {
          ...latest,
          state,
          validEventCount: latest.validEventCount + 1,
        }
      } catch (_error) {
        return { ...latest, malformedLineCount: latest.malformedLineCount + 1 }
      }
    }, {
      state: fallback,
      validEventCount: 0,
      ignoredLineCount: 0,
      malformedLineCount: 0,
    })

  const latestTimestamp = result.validEventCount > 0 ? result.state.timestamp : undefined
  const statusWithoutSummary: Omit<PresenceLedgerStatus, 'summary'> = {
    source: result.validEventCount > 0 ? 'live_ledger' : 'fallback_default',
    freshness: result.validEventCount > 0 ? presenceLedgerFreshness(latestTimestamp, fallbackTimestamp) : 'unknown',
    validEventCount: result.validEventCount,
    ignoredLineCount: result.ignoredLineCount,
    malformedLineCount: result.malformedLineCount,
    ...(result.validEventCount > 0 && result.state.eventId ? { latestEventId: result.state.eventId } : {}),
    ...(latestTimestamp ? { latestTimestamp } : {}),
    ...(() => {
      const ageSeconds = presenceLedgerAgeSeconds(latestTimestamp, fallbackTimestamp)
      return typeof ageSeconds === 'number' ? { ageSeconds } : {}
    })(),
  }

  return {
    state: result.state,
    status: {
      ...statusWithoutSummary,
      summary: presenceLedgerSummary(statusWithoutSummary),
    },
  }
}

export function deriveLatestPresenceStateFromEventLedger(
  ledgerText: string,
  fallbackTimestamp = new Date().toISOString(),
): AgentPresenceState {
  return derivePresenceLedgerProjection(ledgerText, fallbackTimestamp).state
}

const AGENT_VISUAL_COLORS: Record<SovereignAgentId, string> = {
  arandur: '#dffcff',
  athena: '#b98cff',
  hermes: '#6ea8ff',
  manwe: '#42f5d7',
  warden: '#ffb14a',
  oracle: '#ffd76a',
  plutus: '#68f58f',
  apollo: '#ff8ed6',
  mnemosyne: '#9cc7ff',
  prometheus: '#ff6e6e',
  hades: '#8f9bb3',
}

const AGENT_SHORT_LABELS: Record<SovereignAgentId, string> = {
  arandur: 'ARN',
  athena: 'ATH',
  hermes: 'HER',
  manwe: 'CHA',
  warden: 'WRD',
  oracle: 'ORC',
  plutus: 'PLU',
  apollo: 'APL',
  mnemosyne: 'MNE',
  prometheus: 'PRO',
  hades: 'HAD',
}

export function presenceSupportMarkers(state: AgentPresenceState): PresenceSupportMarker[] {
  const focus = new Set(state.focus)
  return state.supportAgents.slice(0, 6).map((agent, index, agents) => {
    const angleRadians = agents.length === 1
      ? -Math.PI / 2
      : -Math.PI / 2 + (index / agents.length) * Math.PI * 2
    return {
      agent,
      label: AGENT_SHORT_LABELS[agent],
      color: AGENT_VISUAL_COLORS[agent],
      angleRadians,
      radius: 0.72 + Math.min(0.18, agents.length * 0.025),
      phaseOffset: index * 0.85,
      isFocus: focus.has(agent),
    }
  })
}

export function presenceVisualState(state: AgentPresenceState): PresenceVisualState {
  if (state.urgency === 'high' || state.scenario === 'alert' || state.phase === 'alert' || state.phase === 'awaiting_user') {
    return {
      pulseRate: 2.15,
      ringOpacity: 1,
      bodyEmissiveIntensity: 3.1,
      scanlineOpacity: 0.34,
      lightIntensity: 1.65,
      supportMarkerScale: 0.88,
    }
  }
  return {
    pulseRate: 1.35,
    ringOpacity: 0.9,
    bodyEmissiveIntensity: 2.5,
    scanlineOpacity: 0.26,
    lightIntensity: 1.25,
    supportMarkerScale: 0.72,
  }
}
