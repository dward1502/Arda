// sigil: REPAIR
import type { AutomationStatusSurface } from './automationStatus'
import type { ArdaSourceProvenance } from './ardaProvenance'
import type { AgentPresenceState, PresenceLedgerStatus } from '../scene/systems/presenceTypes'

export type JsonRecord = Record<string, unknown>

export interface ArdaSection {
  id: string
  title: string
  owner: string
  status: string
  arda_panels: string[]
  primary_sources: string[]
  supplemental_sources?: string[]
  missing_projections?: string[]
}

export interface ArdaSceneZone {
  id: string
  title: string
  scene: 'boardroom' | 'world'
  owner: string
  status: string
  anchor_ids: string[]
  surface_ids: string[]
  workstation_ids: string[]
  source_ids: string[]
}

export interface ArdaSceneAnchor {
  id: string
  scene: 'boardroom' | 'world'
  type: 'monitor' | 'console' | 'control' | 'hologram' | 'gate' | 'district' | 'terminal' | 'workstation_spawn'
  label: string
  zone_id: string
  activation_behavior: 'focus' | 'open_workstation' | 'transition_world' | 'transition_boardroom' | 'open_terminal'
  data_binding: string[]
}

export interface ArdaSceneSurface {
  id: string
  scene: 'boardroom' | 'world'
  kind: 'desk_surface' | 'monitor_preview' | 'console_cluster' | 'district_overlay' | 'terminal_surface'
  label: string
  zone_id: string
  module_ids: string[]
  source_ids: string[]
}

export interface ArdaWorkstationManifest {
  id: string
  title: string
  source_zone_id: string
  entry_anchor_id: string
  module_ids: string[]
  presentation_modes: Array<'in_scene' | 'native_window'>
}

export interface ArdaLedgerState {
  path: string
  label: string
  present: boolean
  recordCount: number
  status: 'ready' | 'empty' | 'missing'
  detail: string
}

export interface ArdaControlLoopTruth {
  wardenGuardhouse: JsonRecord | null
  chronosStatus: JsonRecord | null
  chronosAuditTasks: JsonRecord | null
  ledgerStates: ArdaLedgerState[]
}

export interface ArdaBundle {
  rootPath: string
  generatedAt: string
  settings: JsonRecord | null
  snapshot: JsonRecord | null
  remoteConfidenceSnapshot: JsonRecord | null
  safeLocalWorkCyclePreflight: JsonRecord | null
  l3ReadinessProjection: JsonRecord | null
  flywheelPacketRuntime: JsonRecord | null
  hermesMessages: JsonRecord[]
  flywheelDispatchReceipts: JsonRecord[]
  hermesAgentGatewayReceipts: JsonRecord[]
  agentConversations: JsonRecord[]
  scoutRequests: JsonRecord[]
  scoutFindings: JsonRecord[]
  scoutRuntime: JsonRecord | null
  world: JsonRecord | null
  humanContext: JsonRecord | null
  businessRuntime: JsonRecord | null
  personalRuntime: JsonRecord | null
  runtimeSettings: JsonRecord | null
  configWalkthroughProfiles: JsonRecord | null
  governanceRuntime: JsonRecord | null
  operationsFlow: JsonRecord | null
  soterionRenderContract: JsonRecord | null
  paperclipAlignment: JsonRecord | null
  escalationRuntime: JsonRecord | null
  operatorActions: JsonRecord | null
  outputTopology: JsonRecord | null
  outputAccounting: JsonRecord | null
  packageHealth: JsonRecord | null
  packageEnablement: JsonRecord | null
  packageRuntimeActivation: JsonRecord | null
  storagePressure: JsonRecord | null
  queueSummary: JsonRecord | null
  queueFederation: JsonRecord | null
  fleetRuntimeDrift: JsonRecord | null
  taskLifecycleRuntime: JsonRecord | null
  operatorRuntimeStatus: JsonRecord | null
  humanAugmentationRuntime: JsonRecord | null
  ceoCouncilRuntime: JsonRecord | null
  ceoAutopilotState: JsonRecord | null
  autonomyReadiness: JsonRecord | null
  automationStatus: AutomationStatusSurface | null
  setupConsoleReadiness: JsonRecord | null
  onboardingGuidedSession?: JsonRecord | null
  onboardingPrivateConfigStage?: JsonRecord | null
  onboardingServicePlan?: JsonRecord | null
  auditReadiness: JsonRecord | null
  hadesNightlyOperations: JsonRecord | null
  chronosRuntime: JsonRecord | null
  controlLoopTruth: ArdaControlLoopTruth
  providerIntelligence: JsonRecord | null
  sourceMap: JsonRecord | null
  planMap: JsonRecord | null
  providerTokenUsage: JsonRecord | null
  charonRouter: JsonRecord | null
  athenaRuntime: JsonRecord | null
  taskQueueEntries: JsonRecord[]
  athenaDigest: JsonRecord[]
  athenaDeepGraph: JsonRecord[]
  athenaPolicyReadiness: JsonRecord[]
  knowledgeTriage: JsonRecord[]
  arandurRecommendations: JsonRecord[]
  arandurMissionApprovalRequests: JsonRecord[]
  hadesLifecycleReviewQueue: JsonRecord[]
  agentPresenceState: AgentPresenceState
  agentPresenceStatus: PresenceLedgerStatus
  sections: ArdaSection[]
  sceneZones: ArdaSceneZone[]
  sceneAnchors: ArdaSceneAnchor[]
  sceneSurfaces: ArdaSceneSurface[]
  workstationManifests: ArdaWorkstationManifest[]
  sourceProvenance: ArdaSourceProvenance[]
}

export interface ArdaDataSource {
  readonly name: string
  loadBundle(): Promise<ArdaBundle>
}
