// sigil: REPAIR
import type { WorkstationRoleId } from './workstationRoles'

export type WorkstationStatus = 'ok' | 'attention' | 'empty' | 'unknown'
export type SourceFreshnessStatus = 'fresh' | 'stale' | 'missing' | 'unknown'
export type ActionSafetyClass = 'read_only' | 'dry_run' | 'governed_mutation'

export interface WorkstationMetric {
  id: string
  label: string
  value: number | string
  unit?: string
  tone?: 'neutral' | 'good' | 'attention' | 'critical'
}

export interface WorkstationSourceRef {
  id: string
  label: string
  path?: string
  freshness: {
    status: SourceFreshnessStatus
    timestamp?: string | null
    provenanceClause?: string
  }
}

export interface WorkstationActionDescriptor {
  id: string
  label: string
  safety: ActionSafetyClass
  command?: string
  description?: string
}

export interface ViewModelPreludeContext {
  roleId: WorkstationRoleId
  title: string
  status: WorkstationStatus
  summary: string[]
  metrics: WorkstationMetric[]
  sources: WorkstationSourceRef[]
  actions: WorkstationActionDescriptor[]
}

export interface ViewModelFocusedContext {
  previewKinds: string[]
  focusedCapabilities: string[]
  rawDisclosure: boolean
  purpose?: string
  operatorQuestion?: string
}

export interface ViewModelProvenanceClauseBag {
  provenanceClause?: string
  queueToDecisionEvidence?: string
  sourceToMissingProjectionReason?: string
}

export type FleetProviderTier = 'local' | 'cloud' | 'operator_projection'

export interface FleetProviderModel {
  id: string
  contextWindow: number | null
  healthy: boolean
  isDefault: boolean
  capableTasks: string[]
}

export interface FleetProviderViewModel {
  providerId: string
  providerName: string
  accessTier: FleetProviderTier
  qualityBand: string
  enabled: boolean
  healthy: boolean
  models: FleetProviderModel[]
  avgLatencyMs: number | null
  activeConnections: number
}

export interface FleetLaneOwnershipViewModel {
  lane: string
  priority: string
  route: {
    providerId: string
    modelId: string
    routeClass: string
    reason: string
  } | null
}

export interface FleetLaneHeadroomViewModel {
  providerId: string
  softCaps: Record<'interactive' | 'execution' | 'background', number>
  laneHeadroom: Record<'interactive' | 'execution' | 'background', number>
}

export interface FleetLaneFitnessViewModel {
  lane: string
  providerId: string
  avgLatencyMs: number | null
  successCount: number
  failureCount: number
}

export interface FleetViewModel extends ViewModelPreludeContext, ViewModelFocusedContext, ViewModelProvenanceClauseBag {
  roleId: 'fleet'
  providers: FleetProviderViewModel[]
  laneOwnership: FleetLaneOwnershipViewModel[]
  laneHeadroom: FleetLaneHeadroomViewModel[]
  laneFitness: FleetLaneFitnessViewModel[]
}

export interface WorkViewModel extends ViewModelPreludeContext, ViewModelFocusedContext, ViewModelProvenanceClauseBag {
  roleId: 'work'
  queueToDecisionEvidence?: string
  dryRunAvailability?: boolean
}

export interface DecisionViewModel extends ViewModelPreludeContext, ViewModelFocusedContext, ViewModelProvenanceClauseBag {
  roleId: 'decisions'
  approvalPacketsPending?: number
  oldestPendingWaitMs?: number | null
}

export interface KnowledgeViewModel extends ViewModelPreludeContext, ViewModelFocusedContext, ViewModelProvenanceClauseBag {
  roleId: 'knowledge'
  staleProjectionCount?: number
  missingProjectionReasons?: string[]
  mediaDocsAvailable?: boolean
}

export interface EvidenceViewModel extends ViewModelPreludeContext, ViewModelFocusedContext, ViewModelProvenanceClauseBag {
  roleId: 'evidence'
  sourceToMissingProjectionReason?: string
}

export interface SettingsViewModel extends ViewModelPreludeContext, ViewModelFocusedContext, ViewModelProvenanceClauseBag {
  roleId: 'settings'
  adapterProfileCurrent?: string
  slotAssignmentMode?: string
  exportImportAvailable?: boolean
}

export type WorkstationViewModel =
  | FleetViewModel
  | WorkViewModel
  | DecisionViewModel
  | KnowledgeViewModel
  | EvidenceViewModel
  | SettingsViewModel

function missingSource(id: string, label: string, path?: string): WorkstationSourceRef {
  return {
    id,
    label,
    path,
    freshness: {
      status: 'missing',
      timestamp: null,
      provenanceClause: path ? `projection unavailable: ${path}.` : 'projection unavailable.',
    },
  }
}

export function sourceRef(
  id: string,
  label: string,
  status: SourceFreshnessStatus,
  timestamp?: string | null,
  path?: string,
  provenanceClause?: string,
): WorkstationSourceRef {
  return {
    id,
    label,
    path,
    freshness: {
      status,
      timestamp: timestamp ?? null,
      provenanceClause: provenanceClause ?? missingSource(id, label, path).freshness.provenanceClause,
    },
  }
}

function emptyBase(roleId: WorkstationRoleId, title: string, summary: string[], sources: WorkstationSourceRef[]): ViewModelPreludeContext {
  return {
    roleId,
    title,
    status: 'empty',
    summary,
    metrics: [],
    sources,
    actions: [],
  }
}

export function createEmptyFleetViewModel(summary = ['Fleet projection unavailable.']): FleetViewModel {
  return {
    ...emptyBase('fleet', 'Fleet', summary, [
      missingSource('operator_runtime_status', 'Operator Runtime Status', 'core/state/operator_runtime_status.json'),
      missingSource('charon_router', 'Charon Router', 'core/state/manwe_router.json'),
    ]),
    roleId: 'fleet',
    title: 'Fleet',
    purpose: 'Fleet projection unavailable.',
    operatorQuestion: 'Which providers, routes, and lanes are healthy and available?',
    previewKinds: [],
    focusedCapabilities: [],
    rawDisclosure: false,
    provenanceClause: 'fleet model empty because ARDA projection data was missing.',
    providers: [],
    laneOwnership: [],
    laneHeadroom: [],
    laneFitness: [],
  }
}

export function createEmptyWorkViewModel(summary = ['Work queue projection unavailable.']): WorkViewModel {
  return {
    ...emptyBase('work', 'Work', summary, [
      missingSource('queue_active', 'Queue Active', 'core/state/queue_active.json'),
    ]),
    roleId: 'work',
    title: 'Work',
    purpose: 'Work queue projection unavailable.',
    operatorQuestion: 'What work is active, blocked, scheduled, or owned right now?',
    previewKinds: [],
    focusedCapabilities: [],
    rawDisclosure: false,
    queueToDecisionEvidence: 'none',
    dryRunAvailability: false,
  }
}

export function createEmptyDecisionViewModel(summary = ['Decision gate projection unavailable.']): DecisionViewModel {
  return {
    ...emptyBase('decisions', 'Decisions', summary, [
      missingSource('human_augmentation_runtime', 'Human Augmentation Runtime', 'core/state/human_augmentation_runtime.json'),
    ]),
    roleId: 'decisions',
    title: 'Decisions',
    purpose: 'Decision gate projection unavailable.',
    operatorQuestion: 'What governed actions are waiting for operator judgment?',
    previewKinds: [],
    focusedCapabilities: [],
    rawDisclosure: false,
    approvalPacketsPending: 0,
    oldestPendingWaitMs: null,
  }
}

export function createEmptyKnowledgeViewModel(summary = ['Knowledge projection unavailable.']): KnowledgeViewModel {
  return {
    ...emptyBase('knowledge', 'Knowledge', summary, [
      missingSource('knowledge_triage', 'Knowledge Triage', 'core/projects/athena/knowledge_triage.jsonl'),
    ]),
    roleId: 'knowledge',
    title: 'Knowledge',
    purpose: 'Knowledge projection unavailable.',
    operatorQuestion: 'Which sources, memories, and citations are fresh, stale, or contested?',
    previewKinds: [],
    focusedCapabilities: [],
    rawDisclosure: false,
    staleProjectionCount: 0,
    missingProjectionReasons: [],
    mediaDocsAvailable: false,
  }
}

export function createEmptyEvidenceViewModel(summary = ['Evidence projection unavailable.']): EvidenceViewModel {
  return {
    ...emptyBase('evidence', 'Evidence', summary, [
      missingSource('source_provenance', 'Source Provenance'),
    ]),
    roleId: 'evidence',
    title: 'Evidence',
    purpose: 'Evidence projection unavailable.',
    operatorQuestion: 'What supported or missing evidence backs current claims?',
    previewKinds: [],
    focusedCapabilities: [],
    rawDisclosure: false,
    sourceToMissingProjectionReason: 'source projection unavailable because evidence data was missing.',
  }
}

export function createEmptySettingsViewModel(summary = ['Settings projection unavailable.']): SettingsViewModel {
  return {
    ...emptyBase('settings', 'Settings', summary, [
      missingSource('runtime_settings', 'Runtime Settings', 'core/state/runtime_settings.json'),
    ]),
    roleId: 'settings',
    title: 'Settings',
    purpose: 'Settings projection unavailable.',
    operatorQuestion: 'How is this surface configured, assigned, and connected?',
    previewKinds: [],
    focusedCapabilities: [],
    rawDisclosure: true,
    adapterProfileCurrent: 'unknown',
    slotAssignmentMode: 'unknown',
    exportImportAvailable: false,
  }
}

export function isSafeActionDescriptor(value: unknown): value is WorkstationActionDescriptor {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const record = value as Partial<WorkstationActionDescriptor>
  return typeof record.id === 'string'
    && record.id.length > 0
    && typeof record.label === 'string'
    && record.label.length > 0
    && (record.safety === 'read_only' || record.safety === 'dry_run' || record.safety === 'governed_mutation')
}
