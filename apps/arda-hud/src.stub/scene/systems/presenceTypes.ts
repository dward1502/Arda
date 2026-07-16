// sigil: REPAIR
export type PresenceScenario =
  | 'idle'
  | 'briefing'
  | 'routing'
  | 'knowledge'
  | 'council'
  | 'alert'
  | 'recovery'

export type PresencePhase =
  | 'idle'
  | 'agent_arrival'
  | 'alert'
  | 'awaiting_user'
  | 'action_confirmed'
  | 'resolved'

export type PresenceUrgency = 'normal' | 'high'

export type SovereignAgentId =
  | 'arandur'
  | 'athena'
  | 'hermes'
  | 'charon'
  | 'warden'
  | 'oracle'
  | 'plutus'
  | 'apollo'
  | 'mnemosyne'
  | 'prometheus'
  | 'hades'

export type PresenceSource = 'arda' | 'arda' | 'operator' | 'citadel'

export type PresenceLedgerSource = 'live_ledger' | 'fallback_default'
export type PresenceLedgerFreshness = 'fresh' | 'stale' | 'unknown'

export interface AgentPresenceState {
  scenario: PresenceScenario
  phase: PresencePhase
  primaryAgent: SovereignAgentId
  supportAgents: SovereignAgentId[]
  focus: SovereignAgentId[]
  urgency: PresenceUrgency
  banner?: string
  inquiry?: string
  action?: string
  response?: string
  eventId?: string
  missionId?: string
  anchorTarget?: string
  source: PresenceSource
  timestamp: string
}

export interface PresenceLedgerStatus {
  source: PresenceLedgerSource
  freshness: PresenceLedgerFreshness
  validEventCount: number
  ignoredLineCount: number
  malformedLineCount: number
  latestEventId?: string
  latestTimestamp?: string
  ageSeconds?: number
  summary: string
}

export interface PresenceLedgerProjection {
  state: AgentPresenceState
  status: PresenceLedgerStatus
}

export interface PresenceVisualState {
  pulseRate: number
  ringOpacity: number
  bodyEmissiveIntensity: number
  scanlineOpacity: number
  lightIntensity: number
  supportMarkerScale: number
}

export interface PresenceSupportMarker {
  agent: SovereignAgentId
  label: string
  color: string
  angleRadians: number
  radius: number
  phaseOffset: number
  isFocus: boolean
}
