// sigil: REPAIR
import type { AutomationStatusSurface } from '../../../lib/automationStatus'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../lib/systemActionBus'
import ModuleCard from '../ModuleCard'
import AgentRosterPanel from './systems/AgentRosterPanel'
import AuditReadinessPanel from './systems/AuditReadinessPanel'
import AutomationStatusPanel from './systems/AutomationStatusPanel'
import CharonCapabilityPanel from './systems/CharonCapabilityPanel'
import FleetHealthPanel from './systems/FleetHealthPanel'
import LaneFitnessPanel from './systems/LaneFitnessPanel'
import LaneHeadroomPanel from './systems/LaneHeadroomPanel'
import KnowledgeMapPanel from './systems/KnowledgeMapPanel'
import RuntimeDriftPanel from './systems/RuntimeDriftPanel'
import OperatorCockpitPanel, { type OperatorCockpitSurface } from './systems/OperatorCockpitPanel'
import RoutingActionContractPanel from './systems/RoutingActionContractPanel'
import RoutingOwnershipPanel from './systems/RoutingOwnershipPanel'
import RoutableProvidersPanel, { type RoutableProviderEntry } from './systems/RoutableProvidersPanel'
import SetupConsoleReadinessPanel from './systems/SetupConsoleReadinessPanel'
import SourceTrustPanel from './systems/SourceTrustPanel'
import TaskListViewer from './systems/TaskListViewer'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'
import type { CharonLiveSnapshot } from '../../../lib/charonLive'

interface SystemAgent {
  name: string
  realm: string
  status: string
  trustScore: string
  sigil: string
}

interface SystemsModuleProps {
  agents: SystemAgent[]
  fleetHealth: {
    totalTargets: number
    liveTargets: number
    routableProviders: number
    intentionalOffline: number
    unexpectedOffline: number
    intentionalOfflineTargets: Array<{ displayName: string; providerId: string }>
    unexpectedOfflineTargets: Array<{ displayName: string; providerId: string }>
  }
  laneOwnership: Array<{
    lane: string
    priority: string
    route: {
      providerId: string
      modelId: string
      routeClass: string
      reason: string
    } | null
  }>
  laneHeadroom: Array<{
    providerId: string
    softCaps: Record<string, number>
    laneHeadroom: Record<string, number>
  }>
  laneFitness: Array<{
    lane: string
    providerId: string
    avgLatencyMs: number | null
    successCount: number
    failureCount: number
  }>
  routableProviders: RoutableProviderEntry[]
  charonLiveSnapshot: CharonLiveSnapshot | null
  charonLiveError: string | null
  charonLiveLoading: boolean
  storagePressure: Record<string, unknown> | null
  automationStatus: AutomationStatusSurface | null
  setupConsoleReadiness: Record<string, unknown> | null
  onboardingGuidedSession?: Record<string, unknown> | null
  onboardingPrivateConfigStage?: Record<string, unknown> | null
  onboardingServicePlan?: Record<string, unknown> | null
  auditReadiness: Record<string, unknown> | null
  operatorCockpit: OperatorCockpitSurface
  runtimeDrift: {
    totalNodes: number
    driftedNodes: number
    items: Array<{
      nodeId: string
      displayName: string
      providerId: string
      declaredModel: string
      declaredContextWindow: number | null
      charonContextWindow: number | null
      actualProcessContextWindow: number | null
      declaredVsCharon: boolean
      declaredVsLocalProcess: boolean
      localRuntimeStatus: string
    }>
  }
  knowledgeMap: {
    summary: Array<{ label: string; value: string }>
    entries: Array<{
      path: string
      title: string
      classification: string
      canonicalHome: string
      authority: string
      domain: string
      glyph: string
    }>
    digest: Array<{
      sourceId: string
      title: string
      status: string
      sourceType: string
      tags: string[]
      summary: string
    }>
    deepGraph: Array<{
      sourceId: string
      confidence: string
      triadPassed: boolean
      nodeCount: number
      edgeCount: number
      labels: string[]
    }>
    policyReadiness: Array<{
      sourceId: string
      readiness: string
      confidence: string
      blockers: string[]
      triadPassed: boolean
    }>
    policySummary: {
      status: string
      policyReadyTotal: number
      referenceOnlyTotal: number
      reviewPressureTotal: number
      nextOperatorAction: string
      promotionPreviewAvailable: boolean
      governanceGate: string
    }
    missingProjections: Array<{
      sectionId: string
      sectionTitle: string
      owner: string
      missing: string[]
    }>
  }
  sourceCoverage?: SourceCoverageBadgeState
  sourceProvenance?: ArdaSourceProvenance[]
  actionDescriptors?: SystemActionDescriptor[]
  capabilityStatuses?: SystemActionCapabilityStatus[]
  actionBusyId?: SystemActionId | null
  actionMessage?: string | null
  onRunAction?: (actionId: SystemActionId) => void
  tag?: string
}

export default function SystemsModule({
  agents,
  fleetHealth,
  laneOwnership,
  laneHeadroom,
  laneFitness,
  routableProviders,
  charonLiveSnapshot,
  charonLiveError,
  charonLiveLoading,
  storagePressure,
  automationStatus,
  setupConsoleReadiness,
  onboardingGuidedSession,
  onboardingPrivateConfigStage,
  onboardingServicePlan,
  auditReadiness,
  operatorCockpit,
  runtimeDrift,
  knowledgeMap,
  sourceCoverage,
  sourceProvenance,
  actionDescriptors = [],
  capabilityStatuses = [],
  actionBusyId,
  actionMessage,
  onRunAction,
  tag,
}: SystemsModuleProps) {
  return (
    <ModuleCard title="Systems" eyebrow="Agent baseline" accent="violet" tag={tag} actions={<SourceCoverageBadge coverage={sourceCoverage} />}>
      <div className="systems-module-grid">
        <SourceTrustPanel records={sourceProvenance} />
        <OperatorCockpitPanel surface={operatorCockpit} />
        <TaskListViewer surface={operatorCockpit} />
        <AutomationStatusPanel status={automationStatus} />
        <SetupConsoleReadinessPanel
          readiness={setupConsoleReadiness}
          guidedSession={onboardingGuidedSession}
          privateConfigStage={onboardingPrivateConfigStage}
          servicePlan={onboardingServicePlan}
        />
        <AuditReadinessPanel readiness={auditReadiness} />
        <FleetHealthPanel {...fleetHealth} />
        <RuntimeDriftPanel {...runtimeDrift} />
        <RoutingActionContractPanel
          actionDescriptors={actionDescriptors}
          capabilityStatuses={capabilityStatuses}
          busyActionId={actionBusyId}
          message={actionMessage}
          onRunAction={onRunAction}
        />
        <RoutingOwnershipPanel lanes={laneOwnership} />
        <LaneHeadroomPanel providers={laneHeadroom} />
        <LaneFitnessPanel entries={laneFitness} />
        <CharonCapabilityPanel
          snapshot={charonLiveSnapshot}
          error={charonLiveError}
          loading={charonLiveLoading}
          storagePressure={storagePressure}
        />
        <RoutableProvidersPanel providers={routableProviders} />
        <KnowledgeMapPanel
          {...knowledgeMap}
          actionDescriptors={actionDescriptors}
          capabilityStatuses={capabilityStatuses}
          busyActionId={actionBusyId}
          actionMessage={actionMessage}
          onRunAction={onRunAction}
        />
        <AgentRosterPanel agents={agents} />
      </div>
    </ModuleCard>
  )
}
