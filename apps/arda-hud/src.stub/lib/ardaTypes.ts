// sigil: HEALTH
// React application types are defined here so that `App.tsx` and
// component props stay consistent. This file is the authority for
// App-level shapes: operating surface reports, queue records, node
// health maps, data freshness, and window control contracts.

export type OperatingSurfaceLaneId =
  | 'Now'
  | 'Work'
  | 'Decisions'
  | 'Knowledge'
  | 'Health'
  | 'Business'
  | 'Evidence'
  | 'Settings'

export type OperatingSurfaceLaneStatus = 'ready' | 'partial' | 'gap'

export interface OperatingSurfaceLaneReport {
  lane: OperatingSurfaceLaneId
  current: string
  gap: string
  next: string
  evidence: string[]
  status: OperatingSurfaceLaneStatus
}

export interface CommandConsoleSurface {
  metrics: Array<{ label: string; value: string; tone: 'good' | 'warn' | 'muted' }>
  lanes: Array<{ title: string; value: string; detail: string; status: OperatingSurfaceLaneStatus }>
  workItems: Array<{ id: string; title: string; owner: string; status: string; priority: string; recordClass: string; laneSubclass: string; promotionReceiptRequired: string }>
  messages: Array<{ id: string; source: string; actor: string; intent: string; body: string; timestamp: string }>
  receipts: Array<{ id: string; source: string; status: string; task: string; summary: string; timestamp: string }>
  conversations: Array<{ id: string; topic: string; speaker: string; messageClass: string; summary: string; risk: string; timestamp: string }>
  scoutItems: Array<{ id: string; kind: string; question: string; requester: string; status: string; sourcePolicy: string; timestamp: string }>
  gaps: Array<{ title: string; detail: string }>
}

export interface HumanAugmentationApproval {
  id: string
  decisionClass: string
  approvers: string
  status: string
  note: string
  commandSignature?: string | null
}

export interface ArandurQueueWriteRequest {
  id: string
  missionCandidateId: string
  queueProposalId: string
  title: string
  scope: string
  commandSignature: string
  requester: string
  status: 'draft' | 'pending' | 'approved' | 'rejected'
  evidence: string[]
}

export type ReviewGateKind = 'queue_write' | 'recommendation' | 'mission_approval' | 'hades_lifecycle' | 'athena_policy_readiness'

export interface ReviewGateItem {
  id: string
  kind: ReviewGateKind
  title: string
  source: string
  status: string
  decisionClass: string
  evidence: string
  summary: string
  checklist: string[]
  createdAtUtc?: string
}

export interface ReviewGateDecisionRecordPreview {
  decisionClass: string
  commandSignature: string
  approvers: string
  evidence: string
}

export interface QueueStatusSplit {
  ready: number
  pending: number
  inProgress: number
  blocked: number
}

export interface ProviderQueueItem {
  id: string
  title: string
  owner: string
  status: string
  priority: string
  recordClass?: string
}

export interface HumanGateItem {
  id: string
  title: string
  status: string
  decisionClass: string
}

export interface ProviderHealthItem {
  providerId: string
  queuePosition?: number
  tailAgeMinutes?: number
}

export interface OperatorCockpitSurface {
  queue: {
    openTotal: number
    deliveredTotal?: number
    items: ProviderQueueItem[]
    statusSplit: QueueStatusSplit
  }
  humanGates: {
    blockedTotal: number
    items: HumanGateItem[]
  }
  providerHealth: {
    providers: ProviderHealthItem[]
  }
  charonLanes: {
    activeLanes: number
    laneIds: string[]
    notes: string[]
  }
}

export interface OperandTrackSurface {
  id: string
  title: string
  owner: string
  status: string
  priority: string
}

export interface OperandTrackSurface2 {
  trackId: string
  status: string
  items: string[]
  owner?: string
}

export interface SourceOverviewSurface {
  sourceId: string
  sourcePath: string
  projectionClass: string
  organizationId?: string
  classification?: string
}

export interface SourceOverviewSurface2 {
  sourceId: string
  sourcePath: string
  organizationId?: string
  lane?: string
  projectionStatus?: string
}

export interface PlanningOperandSurface {
  lane: string
  recordClass?: string
  tracks?: OperandTrackSurface2[]
}

export interface SourceCoverageBadgeState {
  status: 'backed' | 'partial' | 'unmapped'
  label: string
  missingCount: number
}

export interface RoutableProviderModel {
  id: string
  contextWindow: number | null
  healthy: boolean
  isDefault: boolean
  capableTasks: string[]
}

export interface RoutableProviderEntry {
  providerId: string
  providerName: string
  accessTier: string
  qualityBand: string
  enabled: boolean
  healthy: boolean
  models: RoutableProviderModel[]
  avgLatencyMs: number | null
  activeConnections: number
}
