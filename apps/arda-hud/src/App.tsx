// sigil: REPAIR
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import {
  Activity,
  BookOpenText,
  Bot,
  Briefcase,
  FolderKanban,
  Shield,
  Sparkles,
  TerminalSquare,
  UserRound,
  X,
} from 'lucide-react'
import {
  BusinessModule,
  ArandurApprovalWorkstation,
  ExecutiveOverviewModule,
  HermesDashboardModule,
  HumanRealmModule,
  LineList,
  LearningLoopSurfaceWrapper,
  MediaLibraryModule,
  ModuleCard,
  OperatingSurfacePlanModule,
  OperationsActionContractPanel,
  PanelWorkspace,
  PersonalOperationsModule,
  PlanningActionContractPanel,
  WorkbenchModule,
  RuntimeModeBadge,
  SceneWorkstation,
  SceneTransitionOverlay,
  PersonalGrowthModule,
  QueueProvenancePanel,
  ReviewGateWorkstation,
  buildReviewGateDecisionRecordPreview,
  ServiceEmbedModule,
  SettingsModule,
  SectionFocusModule,
  SourceCoverageBadge,
  SystemsModule,
  WorldTerminalActionContractPanel,
  WindowControls,
} from './components/arda'
import type {
  OperatingSurfaceNavKey,
  ModuleId,
  ViewMode,
  ThemeOption,
  ThemeId,
  ReviewGateItem,
  SourceCoverageBadgeState,
  RoutableProviderEntry,
  OperatingSurfaceLaneReport
} from './components/arda/types'
import { getArandurQueueWriteRequests, getHumanAugmentationRuntime, getPlanShelf, getReviewGateItems, getRuntimeDrift, getOperatorRuntimeSurface } from './lib/reviewGateDerivation'
import { deriveBoardroomHudInstruments } from './scene/boardroom/boardroomHudInstruments'
import { adaptBoardroomHudSource } from './scene/boardroom/boardroomHudSourceAdapters'
import BoardroomViewport from './scene/boardroom/BoardroomViewport'
import MonitorSurfaceNativeAcceptance from './scene/boardroom/MonitorSurfaceNativeAcceptance'
import MonitorSessionWorkstation from './scene/boardroom/MonitorSessionWorkstation'
import {
  createMonitorSessionWindowConfig,
  findMonitorSessionRecord,
  parseMonitorSessionWorkstationId,
} from './scene/boardroom/monitorSessionWorkstationRoute'
import WorldRuntimeViewport from './scene/world/WorldViewport'
import { calculateWorldDistrictUrgencies } from './scene/world/worldDistrictUrgency'

import {
  createArdaFleetHealth,
  createArdaFleetViewModel,
  type ArdaFleetHealth,
} from './scene/workstations/adapters/ardaAdapter'
import {
  createCoreStateSource,
  type ArdaBundle,
  type ArdaSection,
  type ArdaWorkstationManifest,
  type JsonRecord,
} from './lib/ardaSource'
import { detectArdaRuntimeMode } from './lib/ardaRuntimeMode'
import {
  getSurfaceAdapterManifest,
  getSurfaceAdapterWorkstationManifests,
} from './lib/surfaceAdapterManifests'
import {
  executeSystemAction,
  getSystemActionCapabilityStatuses,
  getSystemActionDescriptors,
  type SystemActionId,
} from './lib/systemActionBus'
import {
  getStoredWorkstationState,
  initWindowBridge,
  syncWorkstationState,
  windowManager,
  type WorkstationBridgeState,
} from './utils/multiWindow'
import { useArdaActionAdapters } from './components/arda/hooks/useArdaActionAdapters'
import { useArdaBundle } from './components/arda/hooks/useArdaBundle'
import { useArdaRuntimePulse } from './components/arda/hooks/useArdaRuntimePulse'
import { useArdaWindowControls } from './components/arda/hooks/useArdaWindowControls'
import { useBoardroomSlotAssignments } from './components/arda/hooks/useBoardroomSlotAssignments'
import { useManweLiveSnapshot } from './components/arda/hooks/useManweLiveSnapshot'
import { useWorldSurfaceAssignments } from './components/arda/hooks/useWorldSurfaceAssignments'
import {
  agentRefreshMonitorLease,
  agentReleaseMonitor,
  agentClaimMonitorSurface,
  agentReleaseMonitorSurface,
  agentRefreshMonitorSurfaceLease,
  MONITOR_SURFACE_REGISTRY_STORAGE_KEY,
  persistMonitorSurfaceRegistry,
  rehydrateMonitorSurfaceRegistry,
  BOARDROOM_MONITOR_SLOT_IDS,
  BOARDROOM_SCENE_SLOT_IDS,
  BOARDROOM_WORKSTATION_ROLE_PROFILES,
  createMonitorSurface,
  dismissMonitorSurface,
  type BoardroomSceneSlotId,
  type MonitorSurfaceRequest,
} from './lib/boardroomSlotSettings'
import {
  createEmptyMonitorRegistryBySlot,
  coerceRuntimeMonitorRegistry,
  createMonitorSurfaceRegistryBridge,
  selectActiveMonitorRecords,
  type MonitorRecordsBySlot,
} from './lib/monitorSurfaceRegistryBridge'
import type { MonitorSurfaceSessionRecord } from './lib/monitorSurfaceContract'
import { claimFromMonitorEvent, normalizeMonitorLeaseExpiry, resolveMonitorContractSlotId, type MonitorClaimChangedEvent } from './scene/boardroom/monitorSurfaceRuntime'
import {
  WORLD_SCENE_SURFACE_IDS,
  WORLD_TERMINAL_SURFACE_IDS,
  type WorldSceneSurfaceId,
} from './lib/worldSurfaceSettings'
import { getString, getNumber, getBoolean, getSectionById, getTimestamp, getSceneZoneById, getWorkstationManifestById, getWorkstationManifestByZoneId, formatMetric, formatBytes, formatPercent, asRecord, asArray, getAgents, getGovernanceRuntimeSignals, getHumanDocs, getHumanNotes, getOperationsFlowSummary, getOutputAccounting, getOutputTopology, getPackageEnablement, getPackageRuntimeActivation, getPaperclipAlignment, getPackageTools, getStoragePressureSummary, getStorageStores, getAutonomyReadinessSummary, getEscalationRuntime, getGovernanceSummary, getOperatorCockpitSurface, getQueueSummary,getOperatorActions } from "./lib/ardaSurfaces"
import type { SceneAnchorDefinition, SceneZoneDefinition, WorkstationManifestDefinition } from './scene/systems/runtimeTypes'
import type { FleetViewModel } from './scene/workstations/viewModels'
import { getCommandConsoleSurface, getCeoCouncilRuntime, getTaskLifecycleRuntime } from "./lib/reviewGateDerivation"
import { getKnowledgeMap, getOperatingSurfaceReports } from "./lib/operatingSurfaceDerivation"
import { sectionToPanelLayout, formatProviderLabel, formatSectionStatus, formatPanelStatus, titleForSectionOrPanel, asModuleId, localStorageOrNull, MODULE_STORAGE_KEY, readStoredModuleOrder } from './lib/settingsLayout'
import { sortedEntries } from './lib/orderedProjection'
import {
  isDerivedRecord,
  provenanceTag,
  statusTone,
  sourceCoverageForPanel,
  sourceCoverageForSections,
  getFloatingWorkstationCenteredLayout,
  getFloatingWorkstationTileLayout,
} from './lib/bundleDerivation'
import { FLOATING_WORKSTATION_BASE_Z_INDEX } from './utils/constants'
import {
  buildWorkstationModuleRole,
  WORKSTATION_FOCUSED_ZONES,
} from './components/arda/modules/fleet/focusedWorkstationModuleHelpers'
import { openHermesRuntimeWindow, ensureHermesRuntimeSpots, describeHermesRuntimeLaunch } from './lib/hermesDashboardLauncher'
import { companyOpsFromProjection } from './lib/companyOps'
const THEMES: ThemeOption[] = [
  { id: 'cyberpunk', label: 'Cyberpunk' },
  { id: 'gibson2', label: 'Gibson 2.0' },
  { id: 'eva', label: 'EVA' },
]

const source = createCoreStateSource()
const runtimeModeStatus = detectArdaRuntimeMode()



const OPERATING_SURFACE_NAV: Array<{
  lane: OperatingSurfaceNavKey
  panelModeKey: string
  subtitle: string
}> = [
    { lane: 'Now', panelModeKey: 'now_command', subtitle: 'mode, attention, active work' },
    { lane: 'Work', panelModeKey: 'planning_and_queue', subtitle: 'queues, jobs, receipts' },
    { lane: 'Decisions', panelModeKey: 'decisions', subtitle: 'human gates and approvals' },
    { lane: 'Knowledge', panelModeKey: 'knowledge_and_reasoning', subtitle: 'research, memory, citations' },
    { lane: 'Health', panelModeKey: 'systems_health', subtitle: 'fleet, providers, drift' },
    { lane: 'Business', panelModeKey: 'business_ops', subtitle: 'clients, projects, readiness' },
    { lane: 'Evidence', panelModeKey: 'evidence_trust', subtitle: 'sources, audits, provenance' },
    { lane: 'Settings', panelModeKey: 'settings', subtitle: 'setup and configuration' },
  ]
interface FloatingWorkstationState {
  id: string
  manifestId: string
  sourceZoneId: string
  originAnchorId: string
  title: string
  presentationMode: 'in_scene' | 'native_window'
  x: number
  y: number
  width: number
  height: number
  zIndex: number
}

function FleetFocusedWorkstationView({ fleetViewModel }: { fleetViewModel: FleetViewModel | null }) {
  if (!fleetViewModel) {
    return (
      <div className="fleet-focused-view fleet-focused-view--empty">
        <span className="fleet-focused-view__eyebrow">Fleet View Model</span>
        <h3>Fleet projection unavailable</h3>
        <p>Waiting for operator runtime and Manwe router projections.</p>
      </div>
    )
  }

  const primaryProvider = fleetViewModel.providers.find((provider) => provider.enabled && provider.healthy)
    ?? fleetViewModel.providers[0]
    ?? null
  const offlineMetric = fleetViewModel.metrics.find((metric) => metric.id === 'unexpected_offline')

  return (
    <div className={`fleet-focused-view fleet-focused-view--${fleetViewModel.status}`}>
      <div className="fleet-focused-view__hero">
        <div>
          <span className="fleet-focused-view__eyebrow">Fleet View Model</span>
          <h3>{fleetViewModel.title}</h3>
          {fleetViewModel.summary.map((line) => <p key={line}>{line}</p>)}
        </div>
        <span className="fleet-focused-view__status">{fleetViewModel.status}</span>
      </div>
      <div className="fleet-focused-view__metrics">
        {fleetViewModel.metrics.map((metric) => (
          <span className={`fleet-focused-view__metric fleet-focused-view__metric--${metric.tone ?? 'neutral'}`} key={metric.id}>
            <b>{metric.value}{metric.unit ?? ''}</b>
            <small>{metric.label}</small>
          </span>
        ))}
      </div>
      <div className="fleet-focused-view__grid">
        <section>
          <h4>Lane Ownership</h4>
          {fleetViewModel.laneOwnership.map((lane) => (
            <div className="fleet-focused-view__row" key={lane.lane}>
              <span>{lane.priority}</span>
              <b>{lane.route ? `${lane.route.providerId} / ${lane.route.modelId}` : 'unassigned'}</b>
            </div>
          ))}
        </section>
        <section>
          <h4>Providers</h4>
          {fleetViewModel.providers.slice(0, 4).map((provider) => (
            <div className="fleet-focused-view__row" key={provider.providerId}>
              <span>{provider.providerName}</span>
              <b>{provider.healthy ? 'healthy' : 'check'} · {provider.models.length} models</b>
            </div>
          ))}
          {fleetViewModel.providers.length === 0 ? <p>No routable provider projection.</p> : null}
        </section>
      </div>
      <div className="fleet-focused-view__footer">
        <span>Primary: {primaryProvider?.providerName ?? 'none'}</span>
        <span>Unexpected offline: {offlineMetric?.value ?? 0}</span>
        <span>Sources: {fleetViewModel.sources.map((sourceRef) => sourceRef.freshness.status).join(' / ')}</span>
      </div>
    </div>
  )
}

export default function App() {
  const searchParams = new URLSearchParams(window.location.search)
  const currentWindowId = searchParams.get('__windowId') ?? 'main'
  const currentWindowRole = searchParams.get('__windowRole')
  const showCustomWindowControls = currentWindowRole !== 'workstation'
  const { closeWindow, minimizeWindow, toggleFullscreen, startDragging } = useArdaWindowControls(currentWindowId)
  const initialWorkstationId = searchParams.get('__workstation')
  const initialMonitorSessionId = parseMonitorSessionWorkstationId(initialWorkstationId)
  const initialSectionId = searchParams.get('__section')
  const initialView = (searchParams.get('__view') as ViewMode | null) ?? 'boardroom'
  const [theme, setTheme] = useState<ThemeId>('gibson2')
  const [activeSectionId, setActiveSectionId] = useState<string | null>(initialSectionId)
  const [moduleOrder, setModuleOrder] = useState<ModuleId[]>(() => readStoredModuleOrder())
  const onBundleLoaded = useCallback((nextBundle: ArdaBundle) => {
    setActiveSectionId((current) => current ?? nextBundle.sections[0]?.id ?? null)
  }, [])
  const { bundle, error, isLoading, refreshBundle } = useArdaBundle({
    source,
    onLoaded: onBundleLoaded,
  })
  const {
    snapshot: manweLiveSnapshot,
    error: manweLiveError,
    isLoading: manweLiveLoading,
  } = useManweLiveSnapshot(5000)
  const {
    assignments: boardroomSceneSlotAssignments,
    setAssignments: setBoardroomSceneSlotAssignments,
    document: boardroomSlotDocument,
    surfaceLayouts: boardroomSurfaceLayouts,
    monitorSlotSources: boardroomMonitorSlotSources,
    claimMonitorSlot: claimBoardroomMonitorSlot,
    releaseMonitorSlot: releaseBoardroomMonitorSlot,
    refreshMonitorSlot: refreshBoardroomMonitorSlot,
    updateSurfaceLayout: updateBoardroomSurfaceLayout,
    updateVisualization: updateBoardroomVisualization,
    exportProfile: exportBoardroomProfile,
    importProfile: importBoardroomProfile,
    resetProfile: resetBoardroomProfile,
    mode: boardroomSlotAssignmentMode,
    message: boardroomSlotAssignmentMessage,
    saveStatus: boardroomSlotSaveStatus,
  } = useBoardroomSlotAssignments(bundle?.rootPath)
  const [monitorRecordsBySlot, setMonitorRecordsBySlot] = useState<MonitorRecordsBySlot>(
    createEmptyMonitorRegistryBySlot,
  )
  const hydratedMonitorClaimsRef = useRef(new Set<string>())
  const monitorClaimEventStateRef = useRef({
    sources: boardroomMonitorSlotSources,
    claim: claimBoardroomMonitorSlot,
    release: releaseBoardroomMonitorSlot,
  })
  monitorClaimEventStateRef.current = {
    sources: boardroomMonitorSlotSources,
    claim: claimBoardroomMonitorSlot,
    release: releaseBoardroomMonitorSlot,
  }
  const boardroomAgentClaims = useMemo(() => BOARDROOM_MONITOR_SLOT_IDS.reduce<Record<string, import('./lib/boardroomSlotSettings').BoardroomAgentClaim | null>>((claims, slotId) => {
    claims[slotId] = boardroomMonitorSlotSources[slotId]?.claim ?? null
    return claims
  }, {}), [boardroomMonitorSlotSources])

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) return
    let unlisten: (() => void) | null = null
    let cancelled = false
    void import('@tauri-apps/api/event').then(({ listen }) => listen<MonitorClaimChangedEvent>(
      'monitor-claim-changed',
      ({ payload }) => {
        if (!BOARDROOM_MONITOR_SLOT_IDS.includes(payload.slotId as typeof BOARDROOM_MONITOR_SLOT_IDS[number])) return
        const slotId = payload.slotId as BoardroomSceneSlotId
        const eventState = monitorClaimEventStateRef.current
        if (!payload.active) {
          eventState.release(slotId, payload.owner)
          return
        }
        const fallbackPreview = eventState.sources[slotId]?.assignment.surface_layout.preview
        if (!fallbackPreview) return
        eventState.claim(slotId, claimFromMonitorEvent(payload, fallbackPreview))
      },
    )).then((dispose) => {
      if (cancelled) dispose()
      else unlisten = dispose
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  useEffect(() => {
    if (initialMonitorSessionId || !('__TAURI_INTERNALS__' in window)) return
    for (const slotId of BOARDROOM_MONITOR_SLOT_IDS) {
      const source = boardroomMonitorSlotSources[slotId]
      const claim = source?.claim
      if (!claim) continue
      const key = `${slotId}:${claim.owner}`
      if (hydratedMonitorClaimsRef.current.has(key)) continue
      hydratedMonitorClaimsRef.current.add(key)
      void rehydrateMonitorSurfaceRegistry().then((registry) => {
        if (registry?.sessions[slotId]) return
        return agentClaimMonitorSurface({
          slotId,
          owner: { kind: 'agent', name: claim.owner },
          initialContent: {
            kind: 'web',
            url: claim.payload_binding,
            display: source.assignment.surface_layout.focus.mode === 'remote_preview' ? 'capture_stream' : 'inline',
            sandboxProfile: 'default',
          },
          ttlMs: 3_600_000,
        })
      }).catch(() => hydratedMonitorClaimsRef.current.delete(key))
    }
  }, [boardroomMonitorSlotSources, initialMonitorSessionId])

  useEffect(() => {
    if (initialMonitorSessionId || !('__TAURI_INTERNALS__' in window)) return
    void rehydrateMonitorSurfaceRegistry().catch(() => undefined)
  }, [initialMonitorSessionId])

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) return
    let cancelled = false
    let bridge: ReturnType<typeof createMonitorSurfaceRegistryBridge> | null = null
    let expiryTimer: ReturnType<typeof setTimeout> | null = null
    const applyRegistry = (registry: Parameters<typeof selectActiveMonitorRecords>[0]) => {
      if (cancelled || !registry) return
      setMonitorRecordsBySlot(selectActiveMonitorRecords(registry))
      if (expiryTimer) clearTimeout(expiryTimer)
      const now = Date.now()
      const nextExpiry = Object.values(registry.sessions)
        .map((session) => new Date(session.lease_expires_at_utc).getTime())
        .filter((expiry) => Number.isFinite(expiry) && expiry > now)
        .sort((a, b) => a - b)[0]
      if (nextExpiry) expiryTimer = setTimeout(() => applyRegistry(registry), Math.max(1, nextExpiry - now + 1))
    }
    const handleRegistryStorage = (event: StorageEvent) => {
      if (event.key !== MONITOR_SURFACE_REGISTRY_STORAGE_KEY || !event.newValue) return
      try {
        const registry = coerceRuntimeMonitorRegistry(JSON.parse(event.newValue))
        if (registry) applyRegistry(registry)
      } catch {
        // Ignore malformed cross-window persistence updates.
      }
    }
    window.addEventListener('storage', handleRegistryStorage)
    void Promise.all([
      import('@tauri-apps/api/core'),
      import('@tauri-apps/api/event'),
    ]).then(([{ invoke }, { listen }]) => {
      if (cancelled) return
      bridge = createMonitorSurfaceRegistryBridge({
        invoke: (command, args) => invoke(command, args),
        listen: (event, handler) => listen(event, handler),
        onRegistry: (registry) => {
          applyRegistry(registry)
          void persistMonitorSurfaceRegistry(registry)
        },
      })
      return bridge.start()
    }).catch(() => undefined)
    return () => {
      cancelled = true
      if (expiryTimer) clearTimeout(expiryTimer)
      window.removeEventListener('storage', handleRegistryStorage)
      void bridge?.stop()
    }
  }, [])
  const {
    assignments: worldSceneSurfaceAssignments,
    surfaceLayouts: worldSurfaceLayouts,
    updateSurfaceLayout: updateWorldSurfaceLayout,
    mode: worldSurfaceAssignmentMode,
    message: worldSurfaceAssignmentMessage,
    saveStatus: worldSurfaceSaveStatus,
  } = useWorldSurfaceAssignments(bundle?.rootPath)
  useArdaActionAdapters(bundle)
  const [editMode, setEditMode] = useState(false)
  const [viewMode, setViewMode] = useState<ViewMode>(initialView)
  const [derivedBusy, setDerivedBusy] = useState(false)
  const [approvalDecisionClass, setApprovalDecisionClass] = useState('provider_reroute')
  const [approvalApprovers, setApprovalApprovers] = useState('aurelius,bacon')
  const [approvalEvidence, setApprovalEvidence] = useState('arda-hud')
  const [approvalNote, setApprovalNote] = useState('ARDA HUD approval')
  const [approvalBusy, setApprovalBusy] = useState(false)
  const [approvalMessage, setApprovalMessage] = useState<string | null>(null)
  const [refreshActionBusyId, setRefreshActionBusyId] = useState<SystemActionId | null>(null)
  const [refreshActionMessage, setRefreshActionMessage] = useState<string | null>(null)
  const [councilObjective, setCouncilObjective] = useState('Stabilize CEO council ingress and memory discipline')
  const [councilLoopClass, setCouncilLoopClass] = useState('lightweight')
  const [councilDecisionClass, setCouncilDecisionClass] = useState('routine_maintenance')
  const [councilParticipants, setCouncilParticipants] = useState('arandur,warden,steward')
  const [councilProposals, setCouncilProposals] = useState('use discord ingress,record sessions to runtime')
  const [councilObjections, setCouncilObjections] = useState('')
  const [councilValidators, setCouncilValidators] = useState('joulework,love_equation')
  const [councilMemoryLanes, setCouncilMemoryLanes] = useState('ceo_private_working,shared_executive')
  const [councilMemoryWrites, setCouncilMemoryWrites] = useState('Discord ingress is canonical only via ARDA state')
  const [councilSynthesis, setCouncilSynthesis] = useState('Proceed with Discord as ingress, keep ARDA state canonical, promote validated conclusions only.')
  const [councilTriadRequired, setCouncilTriadRequired] = useState(false)
  const [councilHumanEscalated, setCouncilHumanEscalated] = useState(false)
  const [councilPromotedPrivateMemory, setCouncilPromotedPrivateMemory] = useState(false)
  const [councilBusy, setCouncilBusy] = useState(false)
  const [councilMessage, setCouncilMessage] = useState<string | null>(null)
  const [panelModeKey, setPanelModeKey] = useState<string | null>(null)
  const [transitionLabel, setTransitionLabel] = useState<string | null>(null)
  const liveRuntime = useArdaRuntimePulse()
  const [floatingWorkstations, setFloatingWorkstations] = useState<FloatingWorkstationState[]>([])
  const [workstationModuleById, setWorkstationModuleById] = useState<Record<string, ModuleId>>(() => {
    if (!initialWorkstationId) return {}
    const stored = getStoredWorkstationState(initialWorkstationId)
    const activeModuleId = asModuleId(stored?.activeModuleId)
    return activeModuleId ? { [initialWorkstationId]: activeModuleId } : {}
  })

  useEffect(() => {
    document.documentElement.dataset.theme = theme
  }, [theme])

  useEffect(() => {
    initWindowBridge()
  }, [])

  useEffect(() => {
    const handleWorkstationSync = (event: Event) => {
      const detail = (event as CustomEvent<WorkstationBridgeState>).detail
      if (!detail?.workstationId || detail.sourceWindowId === currentWindowId) return
      const activeModuleId = asModuleId(detail.activeModuleId)
      if (activeModuleId) {
        setWorkstationModuleById((current) => ({
          ...current,
          [detail.workstationId]: activeModuleId,
        }))
      }
      if (detail.presentationMode === 'native_window' && detail.sourceZoneId && initialWorkstationId === detail.workstationId) {
        setPanelModeKey(detail.sourceZoneId)
        setActiveSectionId(detail.sourceZoneId)
      }
    }
    window.addEventListener('workstation-sync', handleWorkstationSync)
    return () => window.removeEventListener('workstation-sync', handleWorkstationSync)
  }, [currentWindowId, initialWorkstationId])

  useEffect(() => {
    try {
      localStorageOrNull()?.setItem(MODULE_STORAGE_KEY, JSON.stringify(moduleOrder))
    } catch {
      // Module ordering is a convenience preference; restricted storage should not block the HUD.
    }
  }, [moduleOrder])

  const workstationManifests = useMemo(
    () => [
      ...(bundle?.workstationManifests ?? []),
      ...getSurfaceAdapterWorkstationManifests(),
    ],
    [bundle?.workstationManifests],
  )
  const boardroomMonitors = BOARDROOM_MONITOR_SLOT_IDS.map((slotId) => boardroomSceneSlotAssignments[slotId])

  const sceneSlotAssignmentOptions = useMemo(() => {
    const manifestOptions = workstationManifests
      .filter((manifest) => manifest.source_zone_id !== 'settings')
      .map((manifest) => ({
        id: manifest.source_zone_id,
        label: manifest.title.replace(/\s+Workstation$/, ''),
      }))
    const fallbackOptions = boardroomMonitors.map((sectionId) => ({
      id: sectionId,
      label: sectionId.replace(/[-_]/g, ' '),
    })).filter((option) => !manifestOptions.some((manifestOption) => manifestOption.id === option.id))
    return [
      { id: '', label: 'Placeholder' },
      ...manifestOptions,
      ...fallbackOptions,
    ]
  }, [workstationManifests])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (viewMode === 'world' || viewMode === 'panel') {
          runSceneTransition('Returning To Boardroom', 'boardroom')
        } else {
          toggleFullscreen()
        }
      }
      if (e.key === 'ArrowLeft' && (viewMode === 'world' || viewMode === 'panel')) {
        runSceneTransition('Returning To Boardroom', 'boardroom')
      }
      if (e.altKey && e.key === '1') {
        e.preventDefault()
        runSceneTransition('Entering Boardroom', 'boardroom')
      }
      if (e.altKey && e.key === '2') {
        e.preventDefault()
        runSceneTransition('Entering World Mode', 'world')
      }
      if (e.altKey && e.key === '3' && activeSectionId) {
        e.preventDefault()
        runSceneTransition('Opening Focused Panel', 'panel')
      }
      if (e.key === 'Tab' && !e.altKey && !e.ctrlKey && !e.shiftKey) {
        e.preventDefault()
        const views: ViewMode[] = ['boardroom', 'world', 'panel']
        const currentIndex = views.indexOf(viewMode)
        const nextIndex = (currentIndex + 1) % views.length
        const nextView = views[nextIndex]
        const label = nextView === 'boardroom' ? 'Entering Boardroom'
          : nextView === 'world' ? 'Entering World Mode'
            : 'Opening Focused Panel'
        if (nextView === 'panel' && !activeSectionId) {
          return
        }
        runSceneTransition(label, nextView)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [viewMode, activeSectionId, toggleFullscreen])

  useEffect(() => {
    const url = new URL(window.location.href)
    url.searchParams.set('__view', viewMode)
    if (activeSectionId) {
      url.searchParams.set('__section', activeSectionId)
    } else {
      url.searchParams.delete('__section')
    }
    window.history.replaceState({}, '', url.toString())
  }, [activeSectionId, viewMode])

  const activeSection = useMemo(
    () => getSectionById(bundle?.sections ?? [], activeSectionId),
    [bundle, activeSectionId],
  )
  const activeSceneZone = useMemo(
    () => getSceneZoneById(bundle?.sceneZones ?? [], activeSectionId),
    [bundle, activeSectionId],
  )
  const activeWorkstationManifest = useMemo(
    () => getWorkstationManifestByZoneId(workstationManifests, activeSectionId),
    [workstationManifests, activeSectionId],
  )

  const agents = useMemo(() => (bundle ? getAgents(bundle) : []), [bundle])
  const docs = useMemo(() => (bundle ? getHumanDocs(bundle) : []), [bundle])
  const notes = useMemo(() => (bundle ? getHumanNotes(bundle) : []), [bundle])
  const packages = useMemo(() => (bundle ? getPackageTools(bundle) : []), [bundle])
  const packageEnablement = useMemo(
    () => (bundle ? getPackageEnablement(bundle) : { summary: [], tools: [] }),
    [bundle],
  )
  const packageRuntimeActivation = useMemo(() => (bundle ? getPackageRuntimeActivation(bundle) : []), [bundle])
  const stores = useMemo(() => (bundle ? getStorageStores(bundle) : []), [bundle])
  const outputTopology = useMemo(
    () => (bundle ? getOutputTopology(bundle) : { surfaces: [], candidates: [], counts: { dataJsonl: 0, humanMarkdown: 0, historySnapshots: 0 } }),
    [bundle],
  )
  const outputAccounting = useMemo(
    () => (bundle ? getOutputAccounting(bundle) : { mirrorRoot: 'data/accounting/output_mirror', mode: 'mirror_only_non_destructive', summary: [], candidates: [] }),
    [bundle],
  )
  const governance = useMemo(
    () => (bundle ? getGovernanceSummary(bundle) : { ready: false, weights: [], thresholds: [] }),
    [bundle],
  )
  const autonomyReadiness = useMemo(
    () => (bundle ? getAutonomyReadinessSummary(bundle) : { posture: 'unknown', checkpoint: [], evidence: [], nextUnlocks: [] }),
    [bundle],
  )
  const governanceSignals = useMemo(() => (bundle ? getGovernanceRuntimeSignals(bundle) : []), [bundle])
  const operationsFlow = useMemo(() => (bundle ? getOperationsFlowSummary(bundle) : []), [bundle])
  const paperclipAlignment = useMemo(
    () => (bundle ? getPaperclipAlignment(bundle) : { summary: [], domains: [], tasks: [], evidence: [] }),
    [bundle],
  )
  const escalationRuntime = useMemo(() => (bundle ? getEscalationRuntime(bundle) : { summary: [], reasons: [] }), [bundle])
  const operatorActions = useMemo(() => (bundle ? getOperatorActions(bundle) : { summary: [], actions: [] }), [bundle])
  const storagePressure = useMemo(
    () => (bundle ? getStoragePressureSummary(bundle) : { summary: [], roots: [], candidates: [] }),
    [bundle],
  )
  const queueSummary = useMemo(
    () => (bundle ? getQueueSummary(bundle) : { completed: 0, priorities: [], owners: [] }),
    [bundle],
  )
  const operatorRuntime = useMemo(() => (bundle ? getOperatorRuntimeSurface(bundle) : null), [bundle])
  const humanAugmentation = useMemo(
    () => (bundle ? getHumanAugmentationRuntime(bundle) : { summary: [], approvals: [] }),
    [bundle],
  )
  const arandurQueueWriteRequests = useMemo(
    () => (bundle ? getArandurQueueWriteRequests(bundle) : []),
    [bundle],
  )
  const reviewGateItems = useMemo(
    () => (bundle ? getReviewGateItems(bundle, arandurQueueWriteRequests) : []),
    [bundle, arandurQueueWriteRequests],
  )
  const operatorCockpit = useMemo(
    () => (bundle ? getOperatorCockpitSurface(bundle, reviewGateItems) : {
      queue: { openTotal: 0, statusSplit: { ready: 0, pending: 0, inProgress: 0, blocked: 0 }, items: [] },
      humanGates: { blockedTotal: 0, items: [] },
      warden: { effectiveAttention: 0, rawAttention: 0, repeatedNoise: 0, activeRepairFiles: 0, resolvedRepairFiles: 0 },
      chronos: { runnerStatus: 'missing', readyTaskCount: 0, scheduledTaskCount: 0, dueTasks: [] },
      hermes: { gatewayReceiptCount: 0, dispatchReceiptCount: 0, latestReceipts: [] },
      athena: { policyReady: 0, referenceOnly: 0, implementationReady: 0, latest: [] },
      manwe: {
        providerCount: 0,
        availableProviderCount: 0,
        blockedProviderCount: 0,
        cooldownCount: 0,
        budgetPressureCount: 0,
        toolContextFloor: 0,
        warnings: [],
      },
      autonomyGate: { decision: 'unknown', cleanupPacketCount: 0, externalSourceBlockedCount: 0, reasons: [] },
      storageHygiene: { status: 'missing', cleanupCandidateCount: 0, deletedBytes: 0, warnings: [] },
      ledgerGaps: [],
    }),
    [bundle, reviewGateItems],
  )
  const ceoCouncil = useMemo(
    () => (bundle ? getCeoCouncilRuntime(bundle) : { summary: [], sessions: [], validators: [], memoryLanes: [] }),
    [bundle],
  )
  const taskLifecycle = useMemo(
    () => (bundle ? getTaskLifecycleRuntime(bundle) : { summary: [], pipeline: '', disposalCandidates: [] }),
    [bundle],
  )
  const fleetHealth = useMemo(
    () => (bundle ? createArdaFleetHealth(bundle) : {
      totalTargets: 0,
      liveTargets: 0,
      routableProviders: 0,
      intentionalOffline: 0,
      unexpectedOffline: 0,
      intentionalOfflineTargets: [],
      unexpectedOfflineTargets: [],
    }),
    [bundle],
  )
  const fleetViewModel = useMemo(() => (bundle ? createArdaFleetViewModel(bundle) : null), [bundle])
  const laneOwnership = fleetViewModel?.laneOwnership ?? []
  const laneHeadroom = fleetViewModel?.laneHeadroom ?? []
  const laneFitness = fleetViewModel?.laneFitness ?? []
  const routableProviders = fleetViewModel?.providers ?? []
  const runtimeDrift = useMemo(() => (bundle ? getRuntimeDrift(bundle) : { totalNodes: 0, driftedNodes: 0, items: [] }), [bundle])
  const mostConstrainedLane = useMemo(() => {
    const source = Array.isArray(laneHeadroom) ? laneHeadroom : []
    const flattened = source.flatMap((provider) => {
      const laneRecord = provider.laneHeadroom ?? {}
      return Object.keys(laneRecord).sort().map((lane) => {
        const headroom = laneRecord[lane]
        return {
          providerId: provider.providerId,
          lane,
          headroom: Number(headroom ?? 0),
          softCap: Number(provider.softCaps?.[lane] ?? 0),
        }
      })
    })
    return flattened.sort((a, b) => a.providerId.localeCompare(b.providerId) || a.lane.localeCompare(b.lane)).reduce(
      (lowest, entry) => (!lowest || entry.headroom < lowest.headroom ? entry : lowest),
      null as null | { providerId: string; lane: string; headroom: number; softCap: number },
    )
  }, [laneHeadroom])
  const hottestProvider = useMemo(() => {
    return routableProviders.reduce((highest, provider) => {
      if (!highest || provider.activeConnections > highest.activeConnections) return provider
      return highest
    }, null as null | RoutableProviderEntry)
  }, [routableProviders])
  const dominantLaneFitness = useMemo(() => {
    type FitnessLeader = {
      lane: string
      providerId: string
      avgLatencyMs: number | null
      successCount: number
      failureCount: number
      score: number
    }
    return laneFitness.reduce<FitnessLeader | null>((best, entry) => {
      const score = entry.successCount - entry.failureCount
      if (!best || score > best.score) return { ...entry, score }
      return best
    }, null)
  }, [laneFitness])
  const planShelf = useMemo(
    () => (bundle ? getPlanShelf(bundle) : { humanPlanRoot: 'docs/plans', corePlanRoot: 'core/projects/Plans', plans: [] }),
    [bundle],
  )
  const boardroomHudInstruments = useMemo(
    () => {
      const sourceProvenance = bundle?.sourceProvenance ?? []
      const pendingReviewItems = reviewGateItems.filter((item) =>
        item.status !== 'approved' && item.status !== 'rejected').length
      const commandLanes = Object.keys(asRecord(bundle?.operationsFlow) ?? {}).length
      const instruments = deriveBoardroomHudInstruments({
      fleetHealth: {
        liveTargets: fleetHealth.liveTargets,
        totalTargets: fleetHealth.totalTargets,
        routableProviders: fleetHealth.routableProviders,
        unexpectedOffline: fleetHealth.unexpectedOffline,
        intentionalOffline: fleetHealth.intentionalOffline,
        runtimeDrift,
        source: adaptBoardroomHudSource(sourceProvenance, 'fleet'),
      },
      queue: {
        completed: queueSummary.completed,
        priorityBuckets: queueSummary.priorities.length,
        ownerBuckets: queueSummary.owners.length,
        source: adaptBoardroomHudSource(sourceProvenance, 'queue'),
      },
      knowledge: {
        documents: docs.length,
        plans: planShelf.plans.length,
        source: adaptBoardroomHudSource(sourceProvenance, 'knowledge'),
      },
      routing: {
        routableProviders: routableProviders.length,
        activeConnections: hottestProvider?.activeConnections ?? 0,
        constrainedHeadroom: typeof mostConstrainedLane?.headroom === 'number' ? mostConstrainedLane.headroom : null,
        source: adaptBoardroomHudSource(sourceProvenance, 'routing'),
      },
      governance: {
        reviewItems: reviewGateItems.length,
        pendingItems: pendingReviewItems,
        source: adaptBoardroomHudSource(sourceProvenance, 'governance'),
      },
      human: {
        documents: docs.length,
        notes: notes.length,
        source: adaptBoardroomHudSource(sourceProvenance, 'human'),
      },
      dailyCommand: {
        lanes: commandLanes,
        attentionLanes: pendingReviewItems,
        source: adaptBoardroomHudSource(sourceProvenance, 'daily-command'),
      },
      }, boardroomSlotDocument.assignments)
      return Object.fromEntries(Object.entries(instruments).map(([slotId, instrument]) => {
        const configured = boardroomSlotDocument.assignments.find((assignment) => assignment.slot_id === slotId)?.visualization
        return [slotId, configured ? { ...instrument, preset: configured.preset_id } : instrument]
      }))
    },
    [boardroomSlotDocument, bundle?.operationsFlow, bundle?.sourceProvenance, docs.length, fleetHealth, hottestProvider, mostConstrainedLane, notes.length, planShelf.plans.length, queueSummary, reviewGateItems, routableProviders.length, runtimeDrift],
  )
  const worldDistricts = useMemo(
    () =>
      (bundle?.sceneZones ?? [])
        .filter((zone) => zone.scene === 'world')
        .map((zone) => ({
          id: zone.id,
          label: zone.title,
          status: zone.status,
          owner: zone.owner,
        })),
    [bundle],
  )
  const boardroomSceneZones = useMemo(
    () => (bundle?.sceneZones ?? []).filter((zone) => zone.scene === 'boardroom'),
    [bundle],
  )
  const boardroomSceneAnchors = useMemo(
    () => (bundle?.sceneAnchors ?? []).filter((anchor) => anchor.scene === 'boardroom'),
    [bundle],
  )
  const worldSceneZones = useMemo(
    () => (bundle?.sceneZones ?? []).filter((zone) => zone.scene === 'world'),
    [bundle],
  )
  const worldRuntimeZones = useMemo(
    () => worldSceneZones.map((zone): SceneZoneDefinition => ({
      id: zone.id,
      title: zone.title,
      scene: zone.scene,
      owner: zone.owner,
      status: zone.status,
      anchorIds: zone.anchor_ids,
      surfaceIds: zone.surface_ids,
      workstationIds: zone.workstation_ids,
      sourceIds: zone.source_ids,
    })),
    [worldSceneZones],
  )
  const worldSceneAnchors = useMemo(
    () => (bundle?.sceneAnchors ?? []).filter((anchor) => anchor.scene === 'world'),
    [bundle],
  )
  const worldRuntimeAnchors = useMemo(
    () => worldSceneAnchors.map((anchor): SceneAnchorDefinition => ({
      id: anchor.id,
      scene: anchor.scene,
      type: anchor.type,
      label: anchor.label,
      zoneId: anchor.zone_id,
      activationBehavior: anchor.activation_behavior,
      dataBinding: anchor.data_binding,
    })),
    [worldSceneAnchors],
  )
  const worldDistrictUrgencies = useMemo(
    () => (bundle ? calculateWorldDistrictUrgencies(worldRuntimeZones, bundle) : {}),
    [bundle, worldRuntimeZones],
  )
  const worldTerminals = useMemo(
    () => [
      { code: 'TRM-QUEUE', detail: `Completed queue items ${queueSummary.completed}` },
      { code: 'TRM-TOOLS', detail: `Observed tool surfaces ${getNumber(asRecord(bundle?.packageHealth?.summary)?.tools_total, 0)}` },
      { code: 'TRM-STATUS', detail: `System posture ${getString(asRecord(bundle?.world?.system)?.status, 'READY')}` },
    ],
    [bundle, queueSummary.completed],
  )

  const worldSystem = asRecord(bundle?.world?.system)
  const worldMetrics = asRecord(bundle?.world?.metrics)
  const humanCounts = asRecord(asRecord(bundle?.humanContext?.human_portal)?.counts)
  const packageSummary = asRecord(bundle?.packageHealth?.summary)
  const storageAudit = asRecord(bundle?.storagePressure?.audit_storage_pressure)
  const businessRuntime = asRecord(bundle?.businessRuntime)
  const personalRuntime = asRecord(bundle?.personalRuntime)
  const primaryEntrypoint = getString(bundle?.sourceMap?.arda_primary_entrypoint_recommended, 'core/state/arda_snapshot.json')
  const toolsObserved = getNumber(packageSummary?.tools_total, 0)
  const oversizeFiles = getNumber(storageAudit?.oversize_files_gte_100mb, 0)
  const systemStatus = getString(worldSystem?.status, 'READY')

  const snapshotTag = provenanceTag(bundle?.snapshot, 'Snapshot')
  const sourceMapTag = provenanceTag(bundle?.sourceMap, 'Source Map')
  const humanTag = provenanceTag(bundle?.humanContext, 'Human Context')
  const businessTag = provenanceTag(bundle?.businessRuntime, 'Business Runtime')
  const personalTag = provenanceTag(bundle?.personalRuntime, 'Personal Runtime')
  const queueTag = provenanceTag(bundle?.queueSummary, 'Queue Summary')
  const governanceTag = provenanceTag(bundle?.governanceRuntime, 'Governance Runtime')
  const operationsTag = provenanceTag(bundle?.operationsFlow, 'Operations Flow')
  const operatorTag = provenanceTag(bundle?.operatorRuntimeStatus, 'Operator Status')
  const runtimeSettingsTag = provenanceTag(bundle?.runtimeSettings, 'Runtime Settings')
  const planMapTag = provenanceTag(bundle?.planMap, 'Plan Map')
  const knowledgeMap = bundle
    ? getKnowledgeMap(bundle)
    : {
      summary: [{ label: 'Total', value: '0' }],
      entries: [],
      digest: [],
      deepGraph: [],
      policyReadiness: [],
      policySummary: {
        status: 'unknown',
        policyReadyTotal: 0,
        referenceOnlyTotal: 0,
        reviewPressureTotal: 0,
        nextOperatorAction: 'refresh_athena_digest',
        promotionPreviewAvailable: false,
        governanceGate: 'human_review_required',
      },
      missingProjections: [],
    }
  const actionDescriptors = getSystemActionDescriptors()
  const actionCapabilityStatuses = getSystemActionCapabilityStatuses({
    chronosRuntime: bundle?.chronosRuntime,
    providerIntelligence: bundle?.providerIntelligence,
    queueSummary: bundle?.queueSummary,
    setupConsoleReadiness: bundle?.setupConsoleReadiness,
    hadesNightlyOperations: bundle?.hadesNightlyOperations,
    athenaRuntime: bundle?.athenaRuntime,
    knowledgeTriage: bundle?.knowledgeTriage,
  })
  const sourceMapSections = bundle?.sections ?? []
  const activeSectionCoverage = activeSection ? sourceCoverageForSections([activeSection]) : undefined
  const humanRealmCoverage = sourceCoverageForPanel(sourceMapSections, 'human_realm')
  const governanceControlsCoverage = sourceCoverageForPanel(sourceMapSections, 'governance_controls')
  const operationsAndPackagesCoverage = sourceCoverageForPanel(sourceMapSections, 'operations_and_packages')
  const planningCoverage = sourceCoverageForPanel(sourceMapSections, 'planning')
  const executiveOverviewCoverage = sourceCoverageForPanel(sourceMapSections, 'executive_overview')
  const businessCoverage = sourceCoverageForPanel(sourceMapSections, 'business')
  const personalGrowthCoverage = sourceCoverageForPanel(sourceMapSections, 'personal_growth')
  const activeServiceSurfaceManifest = getSurfaceAdapterManifest(panelModeKey ?? activeSectionId)
  const operatingSurfaceReports = bundle
    ? getOperatingSurfaceReports(bundle, reviewGateItems, fleetHealth, knowledgeMap)
    : []
  const commandConsoleSurface = bundle
    ? getCommandConsoleSurface(bundle, reviewGateItems)
    : null
  const operatingSurfaceReportByLane = Object.fromEntries(
    operatingSurfaceReports.map((report) => [report.lane, report]),
  ) as Partial<Record<OperatingSurfaceNavKey, OperatingSurfaceLaneReport>>
  const operatingSurfaceNavItems = OPERATING_SURFACE_NAV.map((item) => ({
    ...item,
    report: operatingSurfaceReportByLane[item.lane],
  }))
  const nowReport = operatingSurfaceReportByLane.Now
  const workReport = operatingSurfaceReportByLane.Work
  const decisionsReport = operatingSurfaceReportByLane.Decisions
  const healthReport = operatingSurfaceReportByLane.Health
  const liveRuntimeRailStatus = liveRuntime?.status?.toLowerCase() === 'healthy'
    ? 'healthy'
    : liveRuntime
      ? 'warning'
      : 'idle'

  const moduleRegistry: Record<ModuleId, { title: string; node: ReactNode }> = {
    personal_operations: {
      title: 'Personal Operations',
      node: <PersonalOperationsModule />,
    },
    operating_surface: {
      title: 'Operating Surface Review',
      node: (
        <div className="split-stack">
          <OperatingSurfacePlanModule
            reports={operatingSurfaceReports}
            actionDescriptors={actionDescriptors}
            capabilityStatuses={actionCapabilityStatuses}
            liveRuntime={liveRuntime}
            remoteConfidenceSnapshot={bundle?.remoteConfidenceSnapshot ?? null}
            safeLocalWorkCyclePreflight={bundle?.safeLocalWorkCyclePreflight ?? null}
            commandConsole={commandConsoleSurface}
            sourceCoverage={sourceCoverageForPanel(sourceMapSections, 'executive_overview')}
            tag={sourceMapTag}
            actionBusyId={refreshActionBusyId}
            actionMessage={refreshActionMessage}
            onRunRefreshAction={(actionId) => void submitRefreshAction(actionId)}
          />
          <WorldTerminalActionContractPanel
            actionDescriptors={actionDescriptors}
            capabilityStatuses={actionCapabilityStatuses}
            busyActionId={refreshActionBusyId}
            message={refreshActionMessage}
            onRunAction={(actionId) => void submitRefreshAction(actionId)}
          />
        </div>
      ),
    },
    executive_overview: {
      title: 'Executive Overview',
      node: (
        <ExecutiveOverviewModule
          authority={getString(bundle?.snapshot?.authority, 'arda_snapshot_projection')}
          loveEquation={formatMetric(getNumber(worldMetrics?.love_equation, 0))}
          activeTasks={formatMetric(getNumber(worldMetrics?.active_tasks, 0))}
          schemaVersion={getString(bundle?.snapshot?.schema_version, 'arda.core.state.v1')}
          sourceCoverage={executiveOverviewCoverage}
          tag={snapshotTag}
        />
      ),
    },
    section_focus: {
      title: activeSection?.title ?? 'Section Focus',
      node: (
        activeSection ? (
          <SectionFocusModule
            title={activeSection.title}
            eyebrow={activeSection.owner}
            accent={statusTone(activeSection.status)}
            status={activeSection.status}
            owner={activeSection.owner}
            panelCount={activeWorkstationManifest?.module_ids.length ?? activeSection.arda_panels.length}
            sourceCount={activeSceneZone?.source_ids.length ?? activeSection.primary_sources.length}
            panels={activeSection.arda_panels}
            primarySources={activeSection.primary_sources}
            sourceCoverage={activeSectionCoverage}
            sourceProvenance={bundle?.sourceProvenance ?? []}
            tag={sourceMapTag}
          />
        ) : (
          <ModuleCard title="Section Focus" eyebrow="No active section" accent="cyan">
            <div className="empty-state">No section available.</div>
          </ModuleCard>
        )
      ),
    },
    human_realm: {
      title: 'Human Realm',
      node: (
        <HumanRealmModule
          docs={docs}
          notes={notes}
          planShelf={planShelf}
          counts={{
            docs: getNumber(humanCounts?.docs_total, 0),
            notes: getNumber(humanCounts?.notes_total, 0),
            summaries: getNumber(humanCounts?.summaries_total, 0),
            arandur: getNumber(humanCounts?.arandur_docs_total, 0),
          }}
          sourceCoverage={humanRealmCoverage}
          sourceProvenance={bundle?.sourceProvenance ?? []}
          tag={humanTag}
        />
      ),
    },
    systems: {
      title: 'Systems',
      node: (
        <SystemsModule
          agents={agents.map((agent) => ({
            ...agent,
            trustScore: formatPercent(agent.trustScore),
          }))}
          fleetHealth={fleetHealth}
          laneOwnership={laneOwnership}
          laneHeadroom={laneHeadroom}
          laneFitness={laneFitness}
          routableProviders={routableProviders}
          manweLiveSnapshot={manweLiveSnapshot}
          manweLiveError={manweLiveError}
          manweLiveLoading={manweLiveLoading}
          storagePressure={bundle?.storagePressure ?? null}
          automationStatus={bundle?.automationStatus ?? null}
          setupConsoleReadiness={bundle?.setupConsoleReadiness ?? null}
          onboardingGuidedSession={bundle?.onboardingGuidedSession ?? null}
          onboardingPrivateConfigStage={bundle?.onboardingPrivateConfigStage ?? null}
          onboardingServicePlan={bundle?.onboardingServicePlan ?? null}
          auditReadiness={bundle?.auditReadiness ?? null}
          operatorCockpit={operatorCockpit}
          runtimeDrift={runtimeDrift}
          knowledgeMap={knowledgeMap}
          sourceCoverage={sourceCoverageForPanel(sourceMapSections, 'systems')}
          sourceProvenance={bundle?.sourceProvenance ?? []}
          actionDescriptors={actionDescriptors}
          capabilityStatuses={actionCapabilityStatuses}
          actionBusyId={refreshActionBusyId}
          actionMessage={refreshActionMessage}
          onRunAction={(actionId) => void submitRefreshAction(actionId)}
          tag={operatorTag}
        />
      ),
    },
    service_embed: {
      title: activeServiceSurfaceManifest?.title ?? 'Service Surface',
      node: (
        <ServiceEmbedModule manifest={activeServiceSurfaceManifest} />
      ),
    },
    media_library: {
      title: 'ARDA Media Library',
      node: (
        <MediaLibraryModule rootPath={bundle?.rootPath ?? null} />
      ),
    },
    governance_controls: {
      title: 'Governance Controls',
      node: (
        <ModuleCard
          title="Governance Controls"
          eyebrow="Adjustable weights"
          accent="ember"
          tag={governanceTag}
          actions={<SourceCoverageBadge coverage={governanceControlsCoverage} />}
        >
          <div className="split-stack">
            <div>
              <div className="module-subtitle"><Shield size={14} /> Weights</div>
              <LineList items={governance.weights.slice(0, 8).map((item) => ({ label: item.label, value: formatMetric(item.value) }))} />
            </div>
            <div>
              <div className="module-subtitle"><FolderKanban size={14} /> Thresholds</div>
              <LineList items={governance.thresholds.map((item) => ({ label: item.label, value: formatMetric(item.value) }))} />
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Shield size={14} /> Human Augmentation</div>
              <LineList items={humanAugmentation.summary.map((item) => ({ label: item.label, value: item.value }))} />
              <div className="document-list compact" style={{ marginTop: 12 }}>
                {humanAugmentation.approvals.slice(0, 4).map((approval) => (
                  <article className="document-list__item" key={approval.id}>
                    <strong>{approval.decisionClass}</strong>
                    <span>{approval.approvers} / {approval.status}</span>
                    <p>{approval.note}</p>
                  </article>
                ))}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><FolderKanban size={14} /> Issue Approval</div>
              <div style={{ display: 'grid', gap: 10 }}>
                <select
                  value={approvalDecisionClass}
                  onChange={(event) => setApprovalDecisionClass(event.target.value)}
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                >
                  <option value="provider_reroute">provider_reroute</option>
                  <option value="strategy_change">strategy_change</option>
                  <option value="pricing_change">pricing_change</option>
                  <option value="customer_commitment">customer_commitment</option>
                  <option value="destructive_delete">destructive_delete</option>
                </select>
                <input
                  value={approvalApprovers}
                  onChange={(event) => setApprovalApprovers(event.target.value)}
                  placeholder="approvers: aurelius,bacon"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <input
                  value={approvalEvidence}
                  onChange={(event) => setApprovalEvidence(event.target.value)}
                  placeholder="evidence: ticket-123,boardroom-note"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <input
                  value={approvalNote}
                  onChange={(event) => setApprovalNote(event.target.value)}
                  placeholder="note"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <button
                  onClick={() => void submitHumanAugmentationApproval()}
                  disabled={approvalBusy}
                  className="rounded border border-[#ff9933] bg-[#ff9933]/10 px-3 py-2 text-sm font-semibold text-[#ffb86b] transition-colors hover:bg-[#ff9933]/20 disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {approvalBusy ? 'Recording...' : 'Record Approval'}
                </button>
                {approvalMessage ? <div className="text-[11px] text-[#b8c4d4]">{approvalMessage}</div> : null}
              </div>
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Shield size={14} /> Autonomy Readiness</div>
              <LineList items={[
                { label: 'Posture', value: autonomyReadiness.posture },
                ...autonomyReadiness.checkpoint.slice(0, 4),
              ]} />
              <div className="document-list compact" style={{ marginTop: 12 }}>
                {autonomyReadiness.evidence.slice(0, 4).map((item) => (
                  <article className="document-list__item" key={`${item.phase}-${item.title}`}>
                    <strong>{item.phase} · {item.title}</strong>
                    <span>{item.status}</span>
                    <p>{item.source}</p>
                  </article>
                ))}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><FolderKanban size={14} /> Next Unlocks</div>
              <div className="document-list compact">
                {autonomyReadiness.nextUnlocks.length > 0 ? autonomyReadiness.nextUnlocks.map((unlock) => (
                  <article className="document-list__item" key={unlock.title}>
                    <strong>{unlock.title}</strong>
                    <span>{unlock.status}</span>
                    <p>{unlock.requires || 'No additional requirements recorded.'}</p>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No unlocks recorded</strong>
                    <p>Autonomy remains governed by the current checkpoint posture.</p>
                  </article>
                )}
              </div>
            </div>
          </div>
          <div style={{ marginTop: 16 }}>
            <ArandurApprovalWorkstation
              approvals={humanAugmentation.approvals}
              queueWriteRequests={arandurQueueWriteRequests}
              busy={approvalBusy}
              message={approvalMessage}
              rootPath={bundle?.rootPath ?? null}
              onApprove={(request) => {
                const item = reviewGateItems.find((candidate) => candidate.id === request.id)
                if (item) void submitReviewGateDecision(item, 'approved')
                else setApprovalMessage(`Review packet unavailable for ${request.id}`)
              }}
              onReject={(request) => {
                const item = reviewGateItems.find((candidate) => candidate.id === request.id)
                if (item) void submitReviewGateDecision(item, 'rejected')
                else setApprovalMessage(`Review packet unavailable for ${request.id}`)
              }}
            />
          </div>
          <div style={{ marginTop: 16 }}>
            <ReviewGateWorkstation
              approvals={humanAugmentation.approvals}
              items={reviewGateItems}
              busy={approvalBusy}
              message={approvalMessage}
              sourceProvenance={bundle?.sourceProvenance ?? []}
              decisionApprovers={approvalApprovers}
              onApprove={(item) => void submitReviewGateDecision(item, 'approved')}
              onReject={(item) => void submitReviewGateDecision(item, 'rejected')}
            />
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Sparkles size={14} /> CEO Council</div>
              <LineList items={ceoCouncil.summary.map((item) => ({ label: item.label, value: item.value }))} />
              <div className="document-list compact" style={{ marginTop: 12 }}>
                {ceoCouncil.sessions.length > 0 ? ceoCouncil.sessions.slice(0, 4).map((session) => (
                  <article className="document-list__item" key={session.id}>
                    <strong>{session.objective}</strong>
                    <span>{session.loopClass} / {session.decisionClass}</span>
                    <p>{session.outcomeStatus}</p>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No council sessions yet</strong>
                    <p>Discord ingress and council recording are ready for first live sessions.</p>
                  </article>
                )}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><BookOpenText size={14} /> Validator Garage</div>
              <LineList items={ceoCouncil.validators.length > 0 ? ceoCouncil.validators : [{ label: 'Pending', value: '0' }]} />
              <div className="module-subtitle" style={{ marginTop: 16 }}><UserRound size={14} /> Memory Lanes</div>
              <LineList items={ceoCouncil.memoryLanes.length > 0 ? ceoCouncil.memoryLanes : [{ label: 'Pending', value: '0' }]} />
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><FolderKanban size={14} /> Record Council Session</div>
              <div style={{ display: 'grid', gap: 10 }}>
                <input
                  value={councilObjective}
                  onChange={(event) => setCouncilObjective(event.target.value)}
                  placeholder="objective"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
                  <select
                    value={councilLoopClass}
                    onChange={(event) => setCouncilLoopClass(event.target.value)}
                    className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                  >
                    <option value="lightweight">lightweight</option>
                    <option value="triad">triad</option>
                  </select>
                  <select
                    value={councilDecisionClass}
                    onChange={(event) => setCouncilDecisionClass(event.target.value)}
                    className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                  >
                    <option value="routine_maintenance">routine_maintenance</option>
                    <option value="provider_reroute">provider_reroute</option>
                    <option value="strategy_change">strategy_change</option>
                    <option value="customer_commitment">customer_commitment</option>
                    <option value="destructive_delete">destructive_delete</option>
                  </select>
                </div>
                <input
                  value={councilParticipants}
                  onChange={(event) => setCouncilParticipants(event.target.value)}
                  placeholder="participants: arandur,warden,steward"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <input
                  value={councilProposals}
                  onChange={(event) => setCouncilProposals(event.target.value)}
                  placeholder="proposals: comma separated"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <input
                  value={councilObjections}
                  onChange={(event) => setCouncilObjections(event.target.value)}
                  placeholder="objections: comma separated"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <input
                  value={councilValidators}
                  onChange={(event) => setCouncilValidators(event.target.value)}
                  placeholder="validators: joulework,love_equation"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <input
                  value={councilMemoryLanes}
                  onChange={(event) => setCouncilMemoryLanes(event.target.value)}
                  placeholder="memory lanes: ceo_private_working,shared_executive"
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <textarea
                  value={councilMemoryWrites}
                  onChange={(event) => setCouncilMemoryWrites(event.target.value)}
                  placeholder="memory writes: one per line, optionally lane:content"
                  rows={3}
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <textarea
                  value={councilSynthesis}
                  onChange={(event) => setCouncilSynthesis(event.target.value)}
                  placeholder="synthesis"
                  rows={3}
                  className="rounded border border-[#334155] bg-[#0f1720] px-3 py-2 text-sm text-[#dbe7f3]"
                />
                <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap' }}>
                  <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: '#b8c4d4' }}>
                    <input type="checkbox" checked={councilTriadRequired} onChange={(event) => setCouncilTriadRequired(event.target.checked)} />
                    Triad required
                  </label>
                  <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: '#b8c4d4' }}>
                    <input type="checkbox" checked={councilHumanEscalated} onChange={(event) => setCouncilHumanEscalated(event.target.checked)} />
                    Human escalated
                  </label>
                  <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: '#b8c4d4' }}>
                    <input type="checkbox" checked={councilPromotedPrivateMemory} onChange={(event) => setCouncilPromotedPrivateMemory(event.target.checked)} />
                    Promoted private memory
                  </label>
                </div>
                <button
                  onClick={() => void submitCeoCouncilSession()}
                  disabled={councilBusy}
                  className="rounded border border-[#6ee7b7] bg-[#6ee7b7]/10 px-3 py-2 text-sm font-semibold text-[#9ff5ce] transition-colors hover:bg-[#6ee7b7]/20 disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {councilBusy ? 'Recording...' : 'Record Council Session'}
                </button>
                {councilMessage ? <div className="text-[11px] text-[#b8c4d4]">{councilMessage}</div> : null}
              </div>
            </div>
          </div>
        </ModuleCard>
      ),
    },
    hermes_dashboard: {
      title: 'Hermes Dashboard',
      node: (
        <HermesDashboardModule
          summary={packageEnablement.summary}
          tools={packageEnablement.tools}
          runtimeSurfaces={packageRuntimeActivation}
          auditReadiness={bundle?.auditReadiness ?? null}
          sourceProvenance={bundle?.sourceProvenance ?? []}
          tag="Hermes: ARDA native"
        />
      ),
    },
    operations_and_packages: {
      title: 'Operations And Packages',
      node: (
        <ModuleCard
          title="Operations And Packages"
          eyebrow="Dependency observation"
          accent="cyan"
          tag={operationsTag}
          actions={<SourceCoverageBadge coverage={operationsAndPackagesCoverage} />}
        >
          <div className="split-stack">
            <div>
              <div className="module-subtitle"><Bot size={14} /> Critical Tools</div>
              <div className="document-list compact">
                {packages.map((tool) => (
                  <article className="document-list__item" key={tool.tool}>
                    <strong>{tool.tool}</strong>
                    <span>{tool.repo}</span>
                    <p>{tool.status} / {tool.version}</p>
                  </article>
                ))}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><Sparkles size={14} /> Enablement — live bundle or design intent</div>
              <LineList items={packageEnablement.summary.length > 0
                ? packageEnablement.summary.map((item) => ({ label: item.label, value: item.value }))
                : [{ label: 'Hermes registry', value: 'DESIGN INTENT — awaiting live tool registry adapter' }]}
              />
              <div className="document-list compact" style={{ marginTop: 12 }}>
                {(packageEnablement.tools.length > 0 ? packageEnablement.tools : [{
                  tool: 'Hermes capability registry',
                  lane: 'operator_tools',
                  state: 'planned',
                  readiness: 'DESIGN INTENT — not live registry data',
                  nextAction: 'replace with Hermes config/tool registry adapter',
                }]).slice(0, 4).map((tool) => (
                  <article className="document-list__item" key={`enable-${tool.tool}`}>
                    <strong>{tool.tool}</strong>
                    <span>{tool.lane} / {tool.state}</span>
                    <p>{tool.readiness} / {tool.nextAction}</p>
                  </article>
                ))}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><TerminalSquare size={14} /> Runtime Activation — live bundle or design intent</div>
              <div className="document-list compact">
                {(packageRuntimeActivation.length > 0 ? packageRuntimeActivation : [{
                  tool: 'Hermes dashboard surface',
                  status: 'planned',
                  detail: 'DESIGN INTENT — live runtime status adapter pending',
                  ok: 'unknown',
                }]).map((tool) => (
                  <article className="document-list__item" key={`runtime-${tool.tool}`}>
                    <strong>{tool.tool}</strong>
                    <span>{tool.status} / {tool.ok}</span>
                    <p>{tool.detail}</p>
                  </article>
                ))}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><Briefcase size={14} /> Storage</div>
              <div className="document-list compact">
                {stores.map((store) => (
                  <article className="document-list__item" key={store.path}>
                    <strong>{store.path.split('/').slice(-2).join('/')}</strong>
                    <span>{store.changed ? 'compacted' : 'stable'}</span>
                    <p>{Math.round(store.bytesAfter / 1024)} KB</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Activity size={14} /> Governance Runtime</div>
              <LineList items={governanceSignals.map((item) => ({ label: item.label, value: item.value }))} />
              <div className="token-cloud" style={{ marginTop: 12 }}>
                <span className="token-chip">JSONL {outputTopology.counts.dataJsonl}</span>
                <span className="token-chip">Human MD {outputTopology.counts.humanMarkdown}</span>
                <span className="token-chip">History {outputTopology.counts.historySnapshots}</span>
              </div>
            </div>
            <div>
              <div className="module-subtitle"><TerminalSquare size={14} /> Output Topology</div>
              <div className="document-list compact">
                {outputTopology.candidates.slice(0, 4).map((candidate) => (
                  <article className="document-list__item" key={candidate.path}>
                    <strong>{candidate.path}</strong>
                    <span>{candidate.priority} / {candidate.action}</span>
                    <p>{candidate.reason} / JW {formatMetric(candidate.estimatedJoulework)}</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
          <div style={{ marginTop: 16 }}>
            <div className="module-subtitle"><Shield size={14} /> Operations Flow</div>
            <LineList items={operationsFlow.map((item) => ({ label: item.label, value: item.value }))} />
          </div>
          <div style={{ marginTop: 16 }}>
            <OperationsActionContractPanel
              actionDescriptors={actionDescriptors}
              capabilityStatuses={actionCapabilityStatuses}
              busyActionId={refreshActionBusyId}
              message={refreshActionMessage}
              onRunAction={(actionId) => void submitRefreshAction(actionId)}
            />
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Sparkles size={14} /> Paperclip Alignment</div>
              <LineList items={paperclipAlignment.summary.map((item) => ({ label: item.label, value: item.value }))} />
              <div className="token-cloud" style={{ marginTop: 12 }}>
                {paperclipAlignment.domains.map((item) => (
                  <span className="token-chip" key={item.label}>{item.label}: {item.value}</span>
                ))}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><BookOpenText size={14} /> Evidence State</div>
              <div className="document-list compact">
                {paperclipAlignment.evidence.slice(0, 6).map((item) => (
                  <article className="document-list__item" key={item.sourceId}>
                    <strong>{item.sourceId}</strong>
                    <span>{item.readiness}</span>
                    <p>confidence {item.confidence}</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
          <div style={{ marginTop: 16 }}>
            <div className="module-subtitle"><FolderKanban size={14} /> Comparative Tasks</div>
            <div className="document-list compact">
              {paperclipAlignment.tasks.map((task) => (
                <article className="document-list__item" key={`${task.owner}-${task.title}`}>
                  <strong>{task.title}</strong>
                  <span>{task.owner}</span>
                  <p>{task.status}</p>
                </article>
              ))}
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Activity size={14} /> Escalation Runtime</div>
              <LineList items={escalationRuntime.summary.map((item) => ({ label: item.label, value: item.value }))} />
            </div>
            <div>
              <div className="module-subtitle"><Shield size={14} /> Reason Buckets</div>
              <LineList items={escalationRuntime.reasons.map((item) => ({ label: item.label, value: item.value }))} />
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><UserRound size={14} /> Human Needed</div>
              <LineList items={operatorActions.summary.map((item) => ({ label: item.label, value: item.value }))} />
            </div>
            <div>
              <div className="module-subtitle"><BookOpenText size={14} /> Action Queue</div>
              <div className="document-list compact">
                {operatorActions.actions.map((item) => (
                  <article className="document-list__item" key={`${item.owner}-${item.title}`}>
                    <strong>{item.title}</strong>
                    <span>{item.owner} / {item.status}</span>
                    <p>{item.note}</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><FolderKanban size={14} /> Output Accounting</div>
              <LineList items={outputAccounting.summary.map((item) => ({ label: item.label, value: item.value }))} />
              <div className="token-cloud" style={{ marginTop: 12 }}>
                <span className="token-chip">{outputAccounting.mode}</span>
                <span className="token-chip">{outputAccounting.mirrorRoot}</span>
              </div>
            </div>
            <div>
              <div className="module-subtitle"><Briefcase size={14} /> Mirror Results</div>
              <div className="document-list compact">
                {outputAccounting.candidates.slice(0, 4).map((candidate) => (
                  <article className="document-list__item" key={candidate.path}>
                    <strong>{candidate.path}</strong>
                    <span>{candidate.status}</span>
                    <p>{candidate.mirrorPath} / {candidate.bytes} MB / skip {candidate.skippedFiles} / gzip {candidate.compressedFiles}</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Activity size={14} /> Storage Pressure</div>
              <LineList items={storagePressure.summary.map((item) => ({ label: item.label, value: item.value }))} />
            </div>
            <div>
              <div className="module-subtitle"><TerminalSquare size={14} /> Reclaim Candidates</div>
              <div className="document-list compact">
                {storagePressure.candidates.map((candidate) => (
                  <article className="document-list__item" key={candidate.path}>
                    <strong>{candidate.path}</strong>
                    <span>{candidate.action}</span>
                    <p>{candidate.bytes}</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
          <div style={{ marginTop: 16 }}>
            <div className="module-subtitle"><Briefcase size={14} /> Workspace Roots</div>
            <div className="document-list compact">
              {storagePressure.roots.map((root) => (
                <article className="document-list__item" key={root.path}>
                  <strong>{root.path}</strong>
                  <span>{root.classification}</span>
                  <p>{root.bytes}</p>
                </article>
              ))}
            </div>
          </div>
        </ModuleCard>
      ),
    },
    planning: {
      title: 'Planning',
      node: (
        <ModuleCard
          title="Planning"
          eyebrow="Task ledger"
          accent="gold"
          tag={queueTag}
          actions={<SourceCoverageBadge coverage={planningCoverage} />}
        >
          <div className="split-stack">
            <div>
              <div className="module-subtitle"><FolderKanban size={14} /> Priority Load</div>
              <LineList items={queueSummary.priorities.map((item) => ({ label: item.label, value: `${item.value}` }))} />
            </div>
            <div>
              <div className="module-subtitle"><Sparkles size={14} /> Owner Spread</div>
              <LineList items={queueSummary.owners.map((item) => ({ label: item.label, value: `${item.value}` }))} />
            </div>
          </div>
          <QueueProvenancePanel records={bundle?.sourceProvenance ?? []} queueFederation={bundle?.queueFederation ?? null} />
          <PlanningActionContractPanel
            actionDescriptors={actionDescriptors}
            capabilityStatuses={actionCapabilityStatuses}
            busyActionId={refreshActionBusyId}
            message={refreshActionMessage}
            onRunAction={(actionId) => void submitRefreshAction(actionId)}
          />
          <WorkbenchModule />
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><BookOpenText size={14} /> Human Plan Shelf</div>
              <div className="document-list compact">
                <article className="document-list__item">
                  <strong>{planShelf.humanPlanRoot}</strong>
                  <p>Readable plan root for graph and operator thought.</p>
                </article>
                {planShelf.plans.slice(0, 3).map((plan) => (
                  <article className="document-list__item" key={`human-plan-${plan.id}`}>
                    <strong>{plan.title}</strong>
                    <span>{plan.humanPlanPath}</span>
                    <p>{plan.owner} / open tasks {plan.openTaskCount}</p>
                  </article>
                ))}
              </div>
            </div>
            <div>
              <div className="module-subtitle"><TerminalSquare size={14} /> Core Quick Refs</div>
              <div className="document-list compact">
                <article className="document-list__item">
                  <strong>{planShelf.corePlanRoot}</strong>
                  <p>Sovereign quick-reference root for runtime and task linkage.</p>
                </article>
                {planShelf.plans.slice(0, 3).map((plan) => (
                  <article className="document-list__item" key={`core-plan-${plan.id}`}>
                    <strong>{plan.owner}</strong>
                    <span>{plan.coreQuickRefPath}</span>
                    <p>{plan.title}</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
          <div className="split-stack" style={{ marginTop: 16 }}>
            <div>
              <div className="module-subtitle"><Activity size={14} /> Lifecycle Contract</div>
              <LineList items={taskLifecycle.summary.map((item) => ({ label: item.label, value: item.value }))} />
              <p style={{ marginTop: 12, fontSize: 12, lineHeight: 1.5, color: '#b8c4d4' }}>
                {taskLifecycle.pipeline}
              </p>
            </div>
            <div>
              <div className="module-subtitle"><TerminalSquare size={14} /> HADES Review Queue</div>
              <div className="document-list compact">
                {taskLifecycle.disposalCandidates.length > 0 ? taskLifecycle.disposalCandidates.slice(0, 5).map((candidate) => (
                  <article className="document-list__item" key={candidate.id}>
                    <strong>{candidate.title}</strong>
                    <span>{candidate.owner} / {candidate.nextPhase}</span>
                    <p>{candidate.marker} ready for disposal review</p>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No disposal candidates</strong>
                    <p>Completed work marked with ↝ will surface here for HADES review.</p>
                  </article>
                )}
              </div>
            </div>
          </div>
        </ModuleCard>
      ),
    },
    learning_loop: {
      title: 'Learning Loop',
      node: <LearningLoopSurfaceWrapper tag={sourceMapTag} />,
    },
    business: {
      title: 'Business',
      node: (
        <BusinessModule
          mode={getString(businessRuntime?.mode, 'unknown')}
          clientCount={getNumber(asRecord(businessRuntime?.counts)?.client_records_total, 0)}
          stateKeyCount={getNumber(asRecord(businessRuntime?.counts)?.state_keys_total, 0)}
          companyViewTitle={getString(asRecord(businessRuntime?.company_view)?.title, 'Company View')}
          companyViewPreview={getString(asRecord(businessRuntime?.company_view)?.body_preview, 'Business readable context will appear here as the business layer matures.')}
          clientPaths={asArray(asRecord(businessRuntime?.highlights)?.client_paths).map((value) => getString(value)).filter(Boolean)}
          stateKeys={asArray(asRecord(businessRuntime?.highlights)?.state_keys).map((value) => getString(value)).filter(Boolean)}
          sourceCoverage={businessCoverage}
          sourceProvenance={bundle?.sourceProvenance ?? []}
          companyOps={companyOpsFromProjection(businessRuntime?.company_ops)}
          tag={businessTag}
        />
      ),
    },
    personal_growth: {
      title: 'Personal Growth',
      node: (
        <PersonalGrowthModule
          name={getString(asRecord(personalRuntime?.identity)?.name, 'Daniel')}
          role={getString(asRecord(personalRuntime?.identity)?.role, 'Founder / Principal')}
          location={getString(asRecord(personalRuntime?.identity)?.location, 'Unknown')}
          priorities={asArray(asRecord(personalRuntime?.highlights)?.priorities).map((value) => getString(value)).filter(Boolean)}
          values={asArray(asRecord(personalRuntime?.highlights)?.values).map((value) => getString(value)).filter(Boolean)}
          researchDomains={asArray(personalRuntime?.research_domains).map((value) => getString(value)).filter(Boolean)}
          creativeDomains={asArray(personalRuntime?.creative_domains).map((value) => getString(value)).filter(Boolean)}
          personalDocsTotal={getNumber(asRecord(personalRuntime?.counts)?.personal_docs_total, 0)}
          onboardPreview={getString(asRecord(personalRuntime?.onboard)?.body_preview, 'Personal and sovereign human context will appear here as the personal layer matures.')}
          sourceCoverage={personalGrowthCoverage}
          sourceProvenance={bundle?.sourceProvenance ?? []}
          tag={personalTag}
        />
      ),
    },
    culture_and_art: {
      title: 'Culture And Art',
      node: (
        <ModuleCard title="Culture And Art" eyebrow="Reserved module" accent="violet">
          <div className="document-list compact">
            <article className="document-list__item">
              <strong>Purpose</strong>
              <p>Creative systems, media, aesthetic references, and cultural memory will be surfaced here.</p>
            </article>
          </div>
        </ModuleCard>
      ),
    },
    settings: {
      title: 'Settings',
      node: (
        <SettingsModule
          theme={theme}
          editMode={editMode}
          viewMode={viewMode}
          themeOptions={THEMES}
          monitorAssignments={BOARDROOM_SCENE_SLOT_IDS.map((slotId) => {
          const sourceZoneId = boardroomSceneSlotAssignments[slotId]
            const adapter = getSurfaceAdapterManifest(sourceZoneId)
            const surfaceLayout = boardroomSurfaceLayouts[slotId]
            const visualization = boardroomSlotDocument.assignments.find((assignment) => assignment.slot_id === slotId)?.visualization
            const isMonitor = BOARDROOM_MONITOR_SLOT_IDS.includes(slotId as typeof BOARDROOM_MONITOR_SLOT_IDS[number])
            return {
              slot: slotId,
              label: sceneSlotAssignmentOptions.find((option) => option.id === sourceZoneId)?.label ?? slotId,
              sourceZoneId,
              componentId: adapter?.id ?? `${sourceZoneId}_workstation`,
              role: isMonitor ? 'upper_monitor' as const : 'desk_surface' as const,
              adapterType: surfaceLayout?.adapter_type,
              previewMode: surfaceLayout?.preview.mode ?? 'component_grid',
              focusMode: surfaceLayout?.focus.mode ?? 'native_window',
              refreshMs: surfaceLayout?.preview.refresh_ms,
              widgetCount: surfaceLayout?.preview.widgets.length,
              embedUrl: surfaceLayout?.embed.url,
              surfaceLayout,
              visualization,
            }
          })}
          roleAssignmentProfiles={BOARDROOM_WORKSTATION_ROLE_PROFILES}
          worldSurfaceAssignments={WORLD_SCENE_SURFACE_IDS.map((surfaceId) => {
            const sourceZoneId = worldSceneSurfaceAssignments[surfaceId]
            const surfaceLayout = worldSurfaceLayouts[surfaceId]
            const isTerminal = WORLD_TERMINAL_SURFACE_IDS.includes(surfaceId as typeof WORLD_TERMINAL_SURFACE_IDS[number])
            return {
              slot: surfaceId,
              label: surfaceId
                .split('_')
                .filter(Boolean)
                .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
                .join(' '),
              sourceZoneId,
              componentId: `${surfaceId}_surface`,
              role: isTerminal ? 'world_terminal' as const : 'world_district' as const,
              adapterType: surfaceLayout?.adapter_type,
              previewMode: surfaceLayout?.preview.mode,
              focusMode: surfaceLayout?.focus.mode,
              refreshMs: surfaceLayout?.preview.refresh_ms,
              widgetCount: surfaceLayout?.preview.widgets.length,
              embedUrl: surfaceLayout?.embed.url,
              surfaceLayout,
            }
          })}
          futureDomains={[
            { title: 'Business', status: 'reserved in module contract' },
            { title: 'Personal Growth', status: 'reserved in module contract' },
            { title: 'Culture And Art', status: 'reserved in module contract' },
          ]}
          configWalkthrough={bundle?.configWalkthroughProfiles ?? null}
          rootPath={bundle?.rootPath ?? null}
          onConfigApplied={() => void refreshBundle()}
          onUpdateBoardroomAssignment={(slotId, sourceZoneId) => {
            setBoardroomSceneSlotAssignments((current) => ({
              ...current,
              [slotId]: sourceZoneId,
            }))
          }}
          onUpdateSurfaceLayout={(slotId, updater) => updateBoardroomSurfaceLayout(slotId as typeof BOARDROOM_SCENE_SLOT_IDS[number], updater)}
          onUpdateVisualization={(slotId, selection) => updateBoardroomVisualization(slotId as typeof BOARDROOM_SCENE_SLOT_IDS[number], selection)}
          onExportProfile={exportBoardroomProfile}
          onImportProfile={importBoardroomProfile}
          onResetProfile={resetBoardroomProfile}
          boardroomPersistence={{
            mode: boardroomSlotAssignmentMode,
            saveStatus: boardroomSlotSaveStatus,
            message: boardroomSlotAssignmentMessage,
          }}
          onUpdateWorldSurfaceLayout={(surfaceId, updater) => updateWorldSurfaceLayout(surfaceId as WorldSceneSurfaceId, updater)}
          onToggleEditMode={() => setEditMode((current) => !current)}
        />
      ),
    },
  }

  const moduleTitles = useMemo(
    () =>
      Object.fromEntries(
        (Object.keys(moduleRegistry) as ModuleId[]).map((id) => [id, moduleRegistry[id].title]),
      ) as Record<ModuleId, string>,
    [moduleRegistry],
  )
  const panelLayout: ModuleId[] = sectionToPanelLayout(panelModeKey ?? activeSectionId)
  const activePanelModules = panelLayout.map((moduleId) => ({
    id: moduleId,
    title: moduleRegistry[moduleId].title,
    node: moduleRegistry[moduleId].node,
  }))
  const activePanelWorkstationManifest = initialWorkstationId
    ? getWorkstationManifestById(workstationManifests, initialWorkstationId)
    : getWorkstationManifestByZoneId(workstationManifests, panelModeKey ?? activeSectionId)
  const activePanelWorkstationId = activePanelWorkstationManifest?.id ?? initialWorkstationId ?? null

  const buildWorkstationModules = (manifest: ArdaWorkstationManifest | null) => {
    const sourceZoneId = manifest?.source_zone_id ?? null
    const rejectedPanelIds = manifest?.rejected_panel_ids ?? []
    const adapterMissingModule = rejectedPanelIds.length > 0 ? {
      id: 'section_focus' as ModuleId,
      title: 'Module Adapter Missing',
      node: (
        <section className="arda-module">
          <h2>Module adapter missing</h2>
          <p>The source-map panel taxonomy is not silently renderable as HUD modules.</p>
          <code>{rejectedPanelIds.join(', ')}</code>
        </section>
      ),
    } : null
    if (sourceZoneId === 'systems_health' || sourceZoneId === 'routing_health' || sourceZoneId === 'sovereign_world') {
      const fleetModule = {
        id: 'systems' as ModuleId,
        title: 'Fleet',
        node: <FleetFocusedWorkstationView fleetViewModel={fleetViewModel} />,
      }
      const supplementalLayout = (manifest?.module_ids.length ? manifest.module_ids : sectionToPanelLayout(sourceZoneId))
        .filter((moduleId): moduleId is ModuleId => moduleId in moduleRegistry && moduleId !== 'systems')
      const modules = [
        fleetModule,
        ...supplementalLayout.map((moduleId) => ({
          id: moduleId,
          title: moduleRegistry[moduleId].title,
          node: moduleRegistry[moduleId].node,
        })),
      ]
      return adapterMissingModule
        ? [...modules.filter((module) => module.id !== adapterMissingModule.id), adapterMissingModule]
        : modules
    }
    if (sourceZoneId === 'settings') {
      return [{
        id: 'settings' as ModuleId,
        title: moduleRegistry.settings.title,
        node: moduleRegistry.settings.node,
      }]
    }
    const layout = (manifest?.module_ids.length ? manifest.module_ids : sectionToPanelLayout(sourceZoneId))
      .filter((moduleId): moduleId is ModuleId => moduleId in moduleRegistry)
    const modules = layout.map((moduleId) => ({
      id: moduleId,
      title: moduleRegistry[moduleId].title,
      node: moduleRegistry[moduleId].node,
    }))
    return adapterMissingModule
      ? [...modules.filter((module) => module.id !== adapterMissingModule.id), adapterMissingModule]
      : modules
  }
  const boardroomDeskSurfaces = {
    left: {
      label: 'Fleet Guard',
      title: `${fleetHealth.liveTargets}/${fleetHealth.totalTargets} live`,
      value: `${fleetHealth.routableProviders} routable / ${fleetHealth.unexpectedOffline} unexpected offline`,
      status: fleetHealth.unexpectedOffline > 0 ? 'attention' : 'stable',
      metrics: [
        { label: 'Live', value: `${fleetHealth.liveTargets}` },
        { label: 'Routable', value: `${fleetHealth.routableProviders}` },
        { label: 'Offline', value: `${fleetHealth.intentionalOffline}/${fleetHealth.unexpectedOffline}` },
      ],
      trace: [
        `UNEXPECTED :: ${fleetHealth.unexpectedOffline}`,
        `INTENTIONAL :: ${fleetHealth.intentionalOffline}`,
      ],
      tag: operatorTag,
    },
    right: {
      label: 'Manwe Router',
      title: `${laneOwnership[0]?.route?.providerId ?? 'unassigned'} / ${laneOwnership[1]?.route?.providerId ?? 'unassigned'}`,
      value: `${laneOwnership[2]?.route?.providerId ?? 'unassigned'} background / ${laneFitness.length} learned lanes`,
      status: getBoolean(asRecord(operatorRuntime?.summary)?.manwe_http_ok, false) ? 'online' : 'check',
      metrics: [
        { label: 'Chat', value: laneOwnership[0]?.route?.providerId ?? 'n/a' },
        { label: 'Code', value: laneOwnership[1]?.route?.providerId ?? 'n/a' },
        { label: 'Bg', value: laneOwnership[2]?.route?.providerId ?? 'n/a' },
      ],
      trace: [
        `FITNESS :: ${laneFitness.length}`,
        `MODE :: ${viewMode.toUpperCase()}`,
      ],
      tag: sourceMapTag,
    },
  }
  const consoleWidgets = [
    { label: 'Mode', value: editMode ? 'Edit' : 'Command' },
    { label: 'Fleet', value: `${fleetHealth.liveTargets}/${fleetHealth.totalTargets} Live` },
    {
      label: 'Pressure',
      value: mostConstrainedLane
        ? `${mostConstrainedLane.lane.toUpperCase()} ${Math.round(mostConstrainedLane.headroom * 100)}%`
        : 'No Data',
    },
  ]
  const consoleSwitches = [
    {
      label: 'Router',
      value: getBoolean(asRecord(operatorRuntime?.summary)?.manwe_http_ok, false) ? 'ONLINE' : 'CHECK',
    },
    {
      label: 'Local Mesh',
      value: hottestProvider
        ? `${formatProviderLabel(hottestProvider.providerId).toUpperCase()} ${hottestProvider.activeConnections} INFLIGHT`
        : (fleetHealth.unexpectedOffline > 0 ? 'DEGRADED' : 'SYNCED'),
    },
    {
      label: 'Recovery',
      value: fleetHealth.unexpectedOfflineTargets[0]?.displayName ?? dominantLaneFitness?.providerId ?? 'CLEAR',
    },
  ]
  const consoleDials = [
    { label: 'Inspect', value: 'FLEET', hotspotId: 'systems_table' },
    { label: 'Route', value: 'MANWE', hotspotId: 'operations' },
    { label: 'Hermes', value: 'TOOLS', hotspotId: 'hermes_dashboard' },
    { label: 'Mesh', value: 'NETWORK', hotspotId: 'network' },
  ]
  const boardroomUpperMonitors = [
    ...boardroomMonitors.map((sectionId, index) => {
      if (sectionId === 'systems_health') {
        return {
          label: `Slot ${String.fromCharCode(65 + index)}`,
          title: 'Fleet Systems',
          detail: `${fleetHealth.liveTargets}/${fleetHealth.totalTargets} live / ${fleetHealth.routableProviders} routable`,
          status: fleetHealth.unexpectedOffline > 0 ? 'attention' : 'live',
          metrics: [
            { label: 'Live', value: `${fleetHealth.liveTargets}` },
            { label: 'Route', value: `${fleetHealth.routableProviders}` },
            { label: 'Off', value: `${fleetHealth.unexpectedOffline}` },
          ],
          trace: [
            `PLANNED :: ${fleetHealth.intentionalOffline}`,
            `RECOVERY :: ${fleetHealth.unexpectedOfflineTargets[0]?.displayName ?? 'CLEAR'}`,
          ],
          tag: operatorTag,
        }
      }
      if (sectionId === 'routing_health') {
        return {
          label: `Slot ${String.fromCharCode(65 + index)}`,
          title: 'Manwe Routing',
          detail: `${laneOwnership[0]?.route?.providerId ?? 'unassigned'} / ${laneOwnership[1]?.route?.providerId ?? 'unassigned'}`,
          status: getBoolean(asRecord(operatorRuntime?.summary)?.manwe_http_ok, false) ? 'live' : 'check',
          metrics: [
            { label: 'Chat', value: formatProviderLabel(laneOwnership[0]?.route?.providerId) },
            { label: 'Code', value: formatProviderLabel(laneOwnership[1]?.route?.providerId) },
            { label: 'Bg', value: formatProviderLabel(laneOwnership[2]?.route?.providerId) },
          ],
          trace: [
            `PRESSURE :: ${mostConstrainedLane ? `${mostConstrainedLane.lane} ${Math.round(mostConstrainedLane.headroom * 100)}%` : 'n/a'}`,
            `FITNESS :: ${dominantLaneFitness ? `${dominantLaneFitness.lane} -> ${dominantLaneFitness.providerId}` : 'learning'}`,
          ],
          tag: sourceMapTag,
        }
      }
      if (sectionId === 'human_realm') {
        return {
          label: `Slot ${String.fromCharCode(65 + index)}`,
          title: 'Human Realm',
          detail: `${getNumber(humanCounts?.docs_total, 0)} docs / ${getNumber(humanCounts?.notes_total, 0)} notes`,
          status: 'live',
          metrics: [
            { label: 'Docs', value: `${getNumber(humanCounts?.docs_total, 0)}` },
            { label: 'Notes', value: `${getNumber(humanCounts?.notes_total, 0)}` },
            { label: 'Arandur', value: `${getNumber(humanCounts?.arandur_docs_total, 0)}` },
          ],
          trace: docs.slice(0, 2).map((doc) => doc.title),
          tag: humanTag,
        }
      }
      if (sectionId === 'planning_and_queue') {
        return {
          label: `Slot ${String.fromCharCode(65 + index)}`,
          title: 'Planning',
          detail: `${queueSummary.completed} completed / ${queueSummary.priorities.length} priorities`,
          status: 'live',
          metrics: [
            { label: 'Done', value: `${queueSummary.completed}` },
            { label: 'Priorities', value: `${queueSummary.priorities.length}` },
            { label: 'Owners', value: `${queueSummary.owners.length}` },
          ],
          trace: queueSummary.owners.slice(0, 2).map((owner) => `${owner.label}:${owner.value}`),
          tag: queueTag,
        }
      }
      if (sectionId === 'business_ops') {
        return {
          label: `Slot ${String.fromCharCode(65 + index)}`,
          title: 'Business',
          detail: `${getString(businessRuntime?.mode, 'unknown')} / ${getNumber(asRecord(businessRuntime?.counts)?.client_records_total, 0)} client records`,
          status: 'ready',
          metrics: [
            { label: 'Clients', value: `${getNumber(asRecord(businessRuntime?.counts)?.client_records_total, 0)}` },
            { label: 'Keys', value: `${getNumber(asRecord(businessRuntime?.counts)?.state_keys_total, 0)}` },
            { label: 'Mode', value: getString(businessRuntime?.mode, 'unknown') },
          ],
          trace: asArray(asRecord(businessRuntime?.highlights)?.client_paths)
            .map((value) => getString(value))
            .filter(Boolean)
            .slice(0, 2),
          tag: businessTag,
        }
      }
      if (sectionId === 'personal_growth') {
        return {
          label: `Slot ${String.fromCharCode(65 + index)}`,
          title: 'Personal',
          detail: `${getString(asRecord(personalRuntime?.identity)?.name, 'Daniel')} / ${getNumber(asRecord(personalRuntime?.counts)?.personal_docs_total, 0)} docs`,
          status: 'ready',
          metrics: [
            { label: 'Docs', value: `${getNumber(asRecord(personalRuntime?.counts)?.personal_docs_total, 0)}` },
            { label: 'Research', value: `${getNumber(asRecord(personalRuntime?.counts)?.research_domains_total, 0)}` },
            { label: 'Creative', value: `${getNumber(asRecord(personalRuntime?.counts)?.creative_domains_total, 0)}` },
          ],
          trace: asArray(asRecord(personalRuntime?.highlights)?.priorities)
            .map((value) => getString(value))
            .filter(Boolean)
            .slice(0, 2),
          tag: personalTag,
        }
      }

      const section = bundle?.sections.find((candidate) => candidate.id === sectionId) ?? null
      const zone = bundle?.sceneZones.find((candidate) => candidate.id === sectionId) ?? null
      const workstationManifest =
        workstationManifests.find((candidate) => candidate.source_zone_id === sectionId) ?? null
      return {
        label: `Slot ${String.fromCharCode(65 + index)}`,
        title: section?.title ?? 'Unassigned',
        detail: formatSectionStatus(section),
        status: section?.status ?? 'idle',
        metrics: [
          { label: 'Workstation', value: `${workstationManifest?.module_ids.length ?? section?.arda_panels.length ?? 0}` },
          { label: 'Sources', value: `${zone?.source_ids.length ?? section?.primary_sources.length ?? 0}` },
          { label: 'Owner', value: section?.owner ?? 'none' },
        ],
        trace: (workstationManifest?.module_ids ?? section?.arda_panels ?? []).slice(0, 2),
        tag: sourceMapTag,
      }
    }),
  ]
  const regenerateDerivedState = async () => {
    setDerivedBusy(true)
    try {
      await refreshBundle()
    } finally {
      setDerivedBusy(false)
    }
  }

  const submitRefreshAction = async (actionId: SystemActionId) => {
    setRefreshActionBusyId(actionId)
    setRefreshActionMessage(null)
    try {
      const result = await executeSystemAction(actionId, {
        source: 'external',
        persona: 'frankyrache',
        mood: 'deploying',
        payload: {
          requested_from: 'arda_operating_surface_refresh_action_flow',
        },
      })
      setRefreshActionMessage(result.ok ? `${actionId} refreshed via ${result.provider}` : `${actionId} failed: ${result.message}`)
      if (result.ok) {
        refreshBundle()
      }
    } finally {
      setRefreshActionBusyId(null)
    }
  }

  const submitReviewGateDecision = async (item: ReviewGateItem, status: 'approved' | 'rejected') => {
    setApprovalBusy(true)
    setApprovalMessage(null)
    const decisionRecord = buildReviewGateDecisionRecordPreview(item, approvalApprovers)
    try {
      const result = await executeSystemAction('approve_human_augmentation', {
        source: 'external',
        persona: 'frankyrache',
        mood: status === 'approved' ? 'success' : 'warning',
        payload: {
          numenor_path: bundle?.rootPath,
          decision_class: decisionRecord.decisionClass,
          command_signature: decisionRecord.commandSignature,
          approvers: decisionRecord.approvers,
          evidence: decisionRecord.evidence,
          note: status === 'approved' ? decisionRecord.approvalNote : decisionRecord.rejectionNote,
          status,
        },
      })
      setApprovalMessage(result.ok ? `${status} ${item.id} via ${result.provider}` : `Review gate decision failed: ${result.message}`)
      if (result.ok) {
        refreshBundle()
      }
    } finally {
      setApprovalBusy(false)
    }
  }

  const submitHumanAugmentationApproval = async () => {
    setApprovalBusy(true)
    setApprovalMessage(null)
    try {
      const result = await executeSystemAction('approve_human_augmentation', {
        source: 'external',
        persona: 'frankyrache',
        mood: 'success',
        payload: {
          numenor_path: bundle?.rootPath,
          decision_class: approvalDecisionClass,
          approvers: approvalApprovers,
          evidence: approvalEvidence,
          note: approvalNote,
        },
      })
      setApprovalMessage(result.ok ? `Recorded via ${result.provider}` : `Approval failed: ${result.message}`)
      if (result.ok) {
        refreshBundle()
      }
    } finally {
      setApprovalBusy(false)
    }
  }

  const submitCeoCouncilSession = async () => {
    setCouncilBusy(true)
    setCouncilMessage(null)
    try {
      const result = await executeSystemAction('record_ceo_council_session', {
        source: 'external',
        persona: 'frankyrache',
        mood: 'success',
        payload: {
          numenor_path: bundle?.rootPath,
          objective: councilObjective,
          loop_class: councilLoopClass,
          decision_class: councilDecisionClass,
          participants: councilParticipants,
          proposals: councilProposals,
          objections: councilObjections,
          validators: councilValidators,
          memory_lanes: councilMemoryLanes,
          memory_writes: councilMemoryWrites,
          synthesis: councilSynthesis,
          triad_required: councilTriadRequired,
          human_escalated: councilHumanEscalated,
          promoted_private_memory: councilPromotedPrivateMemory,
        },
      })
      setCouncilMessage(result.ok ? `Recorded via ${result.provider}` : `Council session failed: ${result.message}`)
      if (result.ok) {
        refreshBundle()
      }
    } finally {
      setCouncilBusy(false)
    }
  }

  const openWorkstationWindow = (manifest: ArdaWorkstationManifest | null) => {
    if (!manifest) return
    const activeModuleId = workstationModuleById[manifest.id] ?? manifest.module_ids[0] ?? null
    syncWorkstationState({
      workstationId: manifest.id,
      sourceZoneId: manifest.source_zone_id,
      originAnchorId: manifest.entry_anchor_id,
      presentationMode: 'native_window',
      activeModuleId: activeModuleId ?? undefined,
      sourceWindowId: currentWindowId,
    })
    const url = new URL(window.location.href)
    url.searchParams.set('__view', 'panel')
    url.searchParams.set('__section', manifest.source_zone_id)
    url.searchParams.set('__workstation', manifest.id)
    url.searchParams.set('__anchor', manifest.entry_anchor_id)
    windowManager.open({
      id: `arda-workstation-${manifest.id}`,
      title: `ARDA Workstation — ${manifest.title}`,
      width: 1440,
      height: 900,
      position: 'center',
      url: url.toString(),
      windowRole: 'workstation',
      workstationId: manifest.id,
      sourceZoneId: manifest.source_zone_id,
      originAnchorId: manifest.entry_anchor_id,
      presentationMode: 'native_window',
    })
  }

  const openBoardroomMonitorSurface = (request: MonitorSurfaceRequest) => {
    if (!('__TAURI_INTERNALS__' in window)) {
      const manifest = getWorkstationManifestByZoneId(workstationManifests, request.sourceZoneId)
      openWorkstationWindow(manifest)
      return
    }
    void createMonitorSurface(request).catch((error) => console.error('Failed to create monitor surface', error))
  }

  const openMonitorSessionWorkstation = (record: MonitorSurfaceSessionRecord) => {
    windowManager.open(createMonitorSessionWindowConfig(record))
  }

  const releaseBoardroomMonitorClaim = (slotId: BoardroomSceneSlotId, owner: string) => {
    releaseBoardroomMonitorSlot(slotId, owner)
    if (!('__TAURI_INTERNALS__' in window)) return
    const monitorSlotId = resolveMonitorContractSlotId(slotId, slotId)
    const session = monitorSlotId ? monitorRecordsBySlot[monitorSlotId] : null
    if (!session || session.owner !== owner) {
      void agentReleaseMonitor(slotId, owner).then((result) => {
        if (result.windowLabel) return dismissMonitorSurface(result.windowLabel)
      }).catch((error) => console.error('Failed to release legacy monitor claim', error))
      return
    }
    void agentReleaseMonitorSurface(session.surface_session_id, { kind: 'agent', name: owner }).then((result) => {
      if (result.ok) persistMonitorSurfaceRegistry(result.registry)
    }).catch((error) => console.error('Failed to release monitor claim', error))
  }

  const refreshBoardroomMonitorClaim = (slotId: BoardroomSceneSlotId, owner: string) => {
    if (!('__TAURI_INTERNALS__' in window)) {
      refreshBoardroomMonitorSlot(slotId, owner, new Date(Date.now() + 300_000).toISOString())
      return
    }
    const monitorSlotId = resolveMonitorContractSlotId(slotId, slotId)
    const session = monitorSlotId ? monitorRecordsBySlot[monitorSlotId] : null
    if (!session || session.owner !== owner) {
      void agentRefreshMonitorLease(slotId, owner).then((result) => {
        const expiry = normalizeMonitorLeaseExpiry(result.leaseExpiresAtUtc)
        if (result.ok && expiry) refreshBoardroomMonitorSlot(slotId, owner, expiry)
      }).catch((error) => console.error('Failed to refresh legacy monitor claim', error))
      return
    }
    void agentRefreshMonitorSurfaceLease(session.surface_session_id, { kind: 'agent', name: owner }, 3_600_000).then((result) => {
      const expiry = normalizeMonitorLeaseExpiry(result.session?.lease_expires_at_utc ?? '')
      if (result.ok) {
        persistMonitorSurfaceRegistry(result.registry)
        if (expiry) refreshBoardroomMonitorSlot(slotId, owner, expiry)
      }
    }).catch((error) => console.error('Failed to refresh monitor claim', error))
  }

  const spawnFloatingWorkstation = (zoneId: string | null) => {
    const manifest = getWorkstationManifestByZoneId(workstationManifests, zoneId)
    if (!manifest) return
    setActiveSectionId(manifest.source_zone_id)
    setPanelModeKey(manifest.source_zone_id)
    setViewMode('boardroom')
    setFloatingWorkstations((current) => {
      const existing = current.find((entry) => entry.manifestId === manifest.id)
      const nextZ = current.reduce((max, item) => Math.max(max, item.zIndex), FLOATING_WORKSTATION_BASE_Z_INDEX) + 1
      if (existing) {
        syncWorkstationState({
          workstationId: manifest.id,
          sourceZoneId: manifest.source_zone_id,
          originAnchorId: manifest.entry_anchor_id,
          presentationMode: 'in_scene',
          activeModuleId: workstationModuleById[manifest.id] ?? manifest.module_ids[0] ?? undefined,
          layout: { zIndex: nextZ },
          sourceWindowId: currentWindowId,
        })
        return current.map((entry) => entry.manifestId === manifest.id ? { ...entry, zIndex: nextZ } : entry)
      }

      const centeredLayout = getFloatingWorkstationCenteredLayout()
      const nextEntry = {
        id: `scene-${manifest.id}`,
        manifestId: manifest.id,
        sourceZoneId: manifest.source_zone_id,
        originAnchorId: manifest.entry_anchor_id,
        title: manifest.title,
        presentationMode: 'in_scene' as const,
        ...centeredLayout,
        zIndex: nextZ,
      }
      syncWorkstationState({
        workstationId: nextEntry.manifestId,
        sourceZoneId: nextEntry.sourceZoneId,
        originAnchorId: nextEntry.originAnchorId,
        presentationMode: 'in_scene',
        activeModuleId: workstationModuleById[nextEntry.manifestId] ?? manifest.module_ids[0] ?? undefined,
        layout: {
          x: nextEntry.x,
          y: nextEntry.y,
          width: nextEntry.width,
          height: nextEntry.height,
          zIndex: nextEntry.zIndex,
        },
        sourceWindowId: currentWindowId,
      })
      return [
        ...current,
        nextEntry,
      ]
    })
  }

  const tileFloatingWorkstations = () => {
    setFloatingWorkstations((current) => {
      const tiled = current.map((entry, index) => {
        const layout = getFloatingWorkstationTileLayout(index, current.length)
        return {
          ...entry,
          ...layout,
          zIndex: Math.max(entry.zIndex, FLOATING_WORKSTATION_BASE_Z_INDEX + index + 1),
        }
      })
      tiled.forEach((entry) => {
        const manifest = getWorkstationManifestById(workstationManifests, entry.manifestId)
        syncWorkstationState({
          workstationId: entry.manifestId,
          sourceZoneId: entry.sourceZoneId,
          originAnchorId: entry.originAnchorId,
          presentationMode: 'in_scene',
          activeModuleId: workstationModuleById[entry.manifestId] ?? manifest?.module_ids[0] ?? undefined,
          layout: {
            x: entry.x,
            y: entry.y,
            width: entry.width,
            height: entry.height,
            zIndex: entry.zIndex,
          },
          sourceWindowId: currentWindowId,
        })
      })
      return tiled
    })
  }

  const handleBoardroomMonitorOpen = (monitorId: string) => {
    if (monitorId === 'desk-left') {
      spawnFloatingWorkstation('systems_health')
      return
    }
    if (monitorId === 'desk-right') {
      spawnFloatingWorkstation('routing_health')
      return
    }
    if (monitorId.startsWith('upper-')) {
      const index = Number.parseInt(monitorId.replace('upper-', ''), 10)
      const sectionId = boardroomMonitors[index] ?? null
      if (sectionId) {
        spawnFloatingWorkstation(sectionId)
      }
    }
  }

  const focusFloatingWorkstation = (id: string) => {
    setFloatingWorkstations((current) => {
      const nextZ = current.reduce((max, item) => Math.max(max, item.zIndex), FLOATING_WORKSTATION_BASE_Z_INDEX) + 1
      const workstation = current.find((entry) => entry.id === id)
      if (workstation) {
        syncWorkstationState({
          workstationId: workstation.manifestId,
          sourceZoneId: workstation.sourceZoneId,
          originAnchorId: workstation.originAnchorId,
          presentationMode: workstation.presentationMode,
          activeModuleId: workstationModuleById[workstation.manifestId],
          layout: { zIndex: nextZ },
          sourceWindowId: currentWindowId,
        })
      }
      return current.map((entry) => entry.id === id ? { ...entry, zIndex: nextZ } : entry)
    })
  }

  const moveFloatingWorkstation = (id: string, x: number, y: number) => {
    setFloatingWorkstations((current) =>
      current.map((entry) => {
        if (entry.id !== id) return entry
        const next = {
          ...entry,
          x: Math.max(24, Math.min(x, window.innerWidth - entry.width - 24)),
          y: Math.max(24, Math.min(y, window.innerHeight - entry.height - 24)),
        }
        syncWorkstationState({
          workstationId: next.manifestId,
          sourceZoneId: next.sourceZoneId,
          originAnchorId: next.originAnchorId,
          presentationMode: next.presentationMode,
          activeModuleId: workstationModuleById[next.manifestId],
          layout: { x: next.x, y: next.y, width: next.width, height: next.height, zIndex: next.zIndex },
          sourceWindowId: currentWindowId,
        })
        return next
      }),
    )
  }

  const resizeFloatingWorkstation = (id: string, width: number, height: number) => {
    setFloatingWorkstations((current) =>
      current.map((entry) => {
        if (entry.id !== id) return entry
        const next = {
          ...entry,
          width: Math.max(300, Math.min(width, window.innerWidth - entry.x - 48)),
          height: Math.max(200, Math.min(height, window.innerHeight - entry.y - 48)),
        }
        syncWorkstationState({
          workstationId: next.manifestId,
          sourceZoneId: next.sourceZoneId,
          originAnchorId: next.originAnchorId,
          presentationMode: next.presentationMode,
          activeModuleId: workstationModuleById[next.manifestId],
          layout: { x: next.x, y: next.y, width: next.width, height: next.height, zIndex: next.zIndex },
          sourceWindowId: currentWindowId,
        })
        return next
      }),
    )
  }

  const closeFloatingWorkstation = (id: string) => {
    setFloatingWorkstations((current) => current.filter((entry) => entry.id !== id))
  }

  const closeAllFloatingWorkstations = () => {
    setFloatingWorkstations([])
  }

  const popoutFloatingWorkstation = (id: string) => {
    const workstation = floatingWorkstations.find((entry) => entry.id === id)
    if (!workstation) return
    const manifest = getWorkstationManifestById(workstationManifests, workstation.manifestId)
    openWorkstationWindow(manifest)
    closeFloatingWorkstation(id)
  }

  const setWorkstationActiveModule = (workstationId: string, moduleId: ModuleId) => {
    setWorkstationModuleById((current) => ({
      ...current,
      [workstationId]: moduleId,
    }))
    const manifest = getWorkstationManifestById(workstationManifests, workstationId)
    syncWorkstationState({
      workstationId,
      sourceZoneId: manifest?.source_zone_id,
      originAnchorId: manifest?.entry_anchor_id,
      presentationMode: initialWorkstationId === workstationId ? 'native_window' : 'in_scene',
      activeModuleId: moduleId,
      sourceWindowId: currentWindowId,
    })
  }

  const runSceneTransition = (label: string, nextView: ViewMode, nextPanelModeKey: string | null = null) => {
    setTransitionLabel(label)
    window.setTimeout(() => {
      setPanelModeKey(nextPanelModeKey)
      setViewMode(nextView)
    }, 220)
    window.setTimeout(() => {
      setTransitionLabel(null)
    }, 760)
  }

  const openOperatingSurfaceLane = (panelModeKey: string, lane: OperatingSurfaceNavKey) => {
    const sectionId = panelModeKey === 'settings' ? activeSectionId : panelModeKey
    setActiveSectionId(sectionId)
    runSceneTransition(`Opening ${lane}`, 'panel', panelModeKey)
  }

  const handleOpenHermesDashboard = () => {
    const dashboardUrl = 'http://127.0.0.1:9119'
    const tryOpenWindow = () => {
      openHermesRuntimeWindow()
        .then((result) => {
          console.info('[boardroom] hermes window launch', result)
        })
        .catch((error) => {
          console.error('[boardroom] hermes window launch failed', error)
        })
    }

    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      tryOpenWindow()
      return
    }

    window.open(dashboardUrl, '_blank', 'noopener')
  }

  const handleOpenHermesCli = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('open_hermes_terminal_window')
    } catch (error) {
      console.error('[boardroom] terminal launch failed', error)
      window.open('http://127.0.0.1:9119', '_blank', 'noopener')
    }
  }

  const handleHotspotActivate = (hotspotId: string): boolean => {
    if (hotspotId === 'city_window') {
      runSceneTransition('Entering World Mode', 'world')
      return true
    }

    if (hotspotId === 'governance_console') {
      spawnFloatingWorkstation('governance_guardhouse')
      return true
    }

    if (hotspotId === 'human_stack') {
      spawnFloatingWorkstation('human_realm')
      return true
    }

    if (hotspotId === 'systems_table') {
      spawnFloatingWorkstation('sovereign_world')
      return true
    }

    if (hotspotId === 'operations') {
      spawnFloatingWorkstation('planning_and_queue')
      return true
    }

    if (hotspotId === 'hermes_dashboard') {
      handleOpenHermesDashboard()
      return true
    }

    if (hotspotId === 'hermes_cli') {
      handleOpenHermesCli()
      return true
    }

    if (hotspotId === 'network') {
      spawnFloatingWorkstation('routing_and_comms')
      return true
    }

    return false
  }

  const handleSceneAnchorActivate = (anchorId: string) => {
    const anchor = (bundle?.sceneAnchors ?? []).find((candidate) => candidate.id === anchorId) ?? null
    if (!anchor) {
      if (!handleHotspotActivate(anchorId)) {
        spawnFloatingWorkstation(anchorId)
      }
      return
    }

    if (anchor.activation_behavior === 'transition_world') {
      runSceneTransition('Entering World Mode', 'world')
      return
    }
    if (anchor.activation_behavior === 'transition_boardroom') {
      runSceneTransition('Returning To Boardroom', 'boardroom')
      return
    }
    if (anchor.activation_behavior === 'open_workstation' || anchor.activation_behavior === 'open_terminal') {
      spawnFloatingWorkstation(anchor.zone_id)
      return
    }

    setActiveSectionId(anchor.zone_id)
  }

  const renderFloatingWorkstation = (workstation: (typeof floatingWorkstations)[number]) => {
    const manifest = getWorkstationManifestById(workstationManifests, workstation.manifestId)
    const section = bundle?.sections.find((candidate) => candidate.id === workstation.sourceZoneId) ?? null
    return (
      <SceneWorkstation
        key={workstation.id}
        id={workstation.id}
        title={workstation.title}
        subtitle={formatPanelStatus(workstation.sourceZoneId, section)}
        x={workstation.x}
        y={workstation.y}
        width={workstation.width}
        height={workstation.height}
        zIndex={workstation.zIndex}
        modules={buildWorkstationModules(manifest)}
        activeModuleId={workstationModuleById[workstation.manifestId] ?? null}
        onFocus={focusFloatingWorkstation}
        onClose={closeFloatingWorkstation}
        onPopout={popoutFloatingWorkstation}
        onMove={moveFloatingWorkstation}
        onResize={resizeFloatingWorkstation}
        onActiveModuleChange={(_id, moduleId) => setWorkstationActiveModule(workstation.manifestId, moduleId)}
      />
    )
  }

  const floatingWorkstationSceneOverlay = floatingWorkstations.length > 0
    ? floatingWorkstations.map(renderFloatingWorkstation)
    : null

  if (initialMonitorSessionId) {
    const record = findMonitorSessionRecord(monitorRecordsBySlot, initialMonitorSessionId)
    return (
      <MonitorSessionWorkstation
        sessionId={initialMonitorSessionId}
        record={record}
        rootPath={bundle?.rootPath ?? null}
      />
    )
  }

  return (
    <div className={`arda-app arda-app--${viewMode}`}>
      {showCustomWindowControls ? (
        <WindowControls
          onMinimize={minimizeWindow}
          onToggleFullscreen={toggleFullscreen}
          onClose={closeWindow}
        />
      ) : null}
      {viewMode !== 'boardroom' ? (
        <div className="keyboard-hints">
          <span className="kbd">Tab</span> Navigate
          <span className="kbd">Esc</span> Back
          <span className="kbd">Alt+1/2/3</span> Views
        </div>
      ) : null}
      <div className="arda-background" />
      {error ? <div className="arda-error">{error}</div> : null}
      {isLoading && !bundle ? <div className="arda-loading">Loading core-state bundle...</div> : null}
      <MonitorSurfaceNativeAcceptance source={boardroomMonitorSlotSources.monitor_1} />

      {/* <nav className="operating-surface-rail" aria-label="ARDA operating surface navigation">
        <div className="operating-surface-rail__brief">
          <span className="operating-surface-rail__eyebrow">Operating Surface</span>
          <strong>Now: {nowReport?.status ?? 'loading'}</strong>
          <span title={nowReport?.current ?? 'Waiting for core-state bundle.'}>
            {nowReport?.current ?? 'Waiting for core-state bundle.'}
          </span>
        </div>
        <div className="operating-surface-rail__attention" aria-label="Current operator attention">
          <span>Work: {workReport?.status ?? 'loading'}</span>
          <span>Decisions: {decisionsReport?.status ?? 'loading'}</span>
          <span>Health: {healthReport?.status ?? 'loading'}</span>
          <RuntimeModeBadge status={runtimeModeStatus} />
          <span
            className={`operating-surface-rail__pulse operating-surface-rail__pulse--${liveRuntimeRailStatus}`}
            title={liveRuntime
              ? `${liveRuntime.source} event ${liveRuntime.sequence} at ${liveRuntime.lastEventIso}`
              : 'Waiting for live runtime channel pulse'}
          >
            Pulse: {liveRuntime ? `${liveRuntime.status} #${liveRuntime.sequence}` : 'idle'}
          </span>
        </div>
        <div className="operating-surface-rail__lanes">
          {operatingSurfaceNavItems.map((item) => {
            const status = item.report?.status ?? 'partial'
            const isActive = panelModeKey === item.panelModeKey

            return (
              <button
                type="button"
                key={item.lane}
                className={`operating-surface-rail__lane operating-surface-rail__lane--${status}`}
                onClick={() => openOperatingSurfaceLane(item.panelModeKey, item.lane)}
                aria-label={`Open ${item.lane}: ${item.subtitle}. Status ${status}.`}
                aria-current={isActive ? 'page' : undefined}
                data-status={status}
                title={`${item.lane}: ${item.subtitle} — ${item.report?.current ?? 'waiting for report'}`}
              >
                <span>{item.lane}</span>
                <small>{item.subtitle}</small>
              </button>
            )
          })}
        </div>
      </nav> */}

      {viewMode === 'boardroom' ? (
        <>
          <BoardroomViewport
            active
            debug={editMode}
            zones={boardroomSceneZones.map((zone): SceneZoneDefinition => ({
              id: zone.id,
              title: zone.title,
              scene: zone.scene,
              owner: zone.owner,
              status: zone.status,
              anchorIds: zone.anchor_ids,
              surfaceIds: zone.surface_ids,
              workstationIds: zone.workstation_ids,
              sourceIds: zone.source_ids,
            }))}
            anchors={boardroomSceneAnchors.map((anchor): SceneAnchorDefinition => ({
              id: anchor.id,
              scene: anchor.scene,
              type: anchor.type,
              label: anchor.label,
              zoneId: anchor.zone_id,
              activationBehavior: anchor.activation_behavior,
              dataBinding: anchor.data_binding,
            }))}
            workstations={workstationManifests.map((workstation): WorkstationManifestDefinition => ({
              id: workstation.id,
              title: workstation.title,
              sourceZoneId: workstation.source_zone_id,
              entryAnchorId: workstation.entry_anchor_id,
              moduleIds: workstation.module_ids,
              presentationModes: workstation.presentation_modes,
            }))}
            slotAssignments={boardroomSceneSlotAssignments}
            surfaceLayouts={boardroomSurfaceLayouts}
            monitorSlotSources={boardroomMonitorSlotSources}
            monitorRecordsBySlot={monitorRecordsBySlot}
            agentClaims={boardroomAgentClaims}
            onReleaseMonitor={releaseBoardroomMonitorClaim}
            onRefreshMonitor={refreshBoardroomMonitorClaim}
            sourceProvenance={bundle?.sourceProvenance ?? []}
            instruments={boardroomHudInstruments}
            fleetViewModel={fleetViewModel}
            presenceState={bundle?.agentPresenceState}
            presenceStatus={bundle?.agentPresenceStatus}
            rootPath={bundle?.rootPath ?? null}
            sceneOverlay={floatingWorkstationSceneOverlay}
            onActivate={handleSceneAnchorActivate}
            onOpenWorkstation={spawnFloatingWorkstation}
            onOpenMonitorSurface={openBoardroomMonitorSurface}
            onOpenMonitorSession={openMonitorSessionWorkstation}
            onOpenHermesDashboard={handleOpenHermesDashboard}
            onOpenHermesCli={handleOpenHermesCli}
            onOpenSettings={() => spawnFloatingWorkstation('settings')}
          />
          {floatingWorkstations.length > 0 ? (
            <div className="workstation-dock">
              <div className="workstation-dock__header">
                <div className="workstation-dock__label">Open Workstations</div>
                <button type="button" className="workstation-dock__tile" onClick={tileFloatingWorkstations}>
                  Tile
                </button>
              </div>
              <div className="workstation-dock__items">
                {floatingWorkstations
                  .slice()
                  .sort((a, b) => b.zIndex - a.zIndex)
                  .map((workstation) => (
                    <div className="workstation-dock__item" key={`dock-${workstation.id}`}>
                      <button
                        type="button"
                        className="workstation-dock__focus"
                        onClick={() => focusFloatingWorkstation(workstation.id)}
                      >
                        {workstation.title}
                      </button>
                      <button
                        type="button"
                        className="workstation-dock__close"
                        onClick={() => closeFloatingWorkstation(workstation.id)}
                        aria-label={`Close ${workstation.title}`}
                      >
                        <X size={12} />
                      </button>
                    </div>
                  ))}
              </div>
              <button type="button" className="workstation-dock__clear" onClick={closeAllFloatingWorkstations}>
                Clear All
              </button>
            </div>
          ) : null}
          {editMode ? (
            <section className="boardroom-edit-overlay">
              <div className="boardroom-edit-console">
                <div className="boardroom-edit-console__header">
                  <span className="boardroom-edit-console__eyebrow">Operator Edit</span>
                  <strong className="boardroom-edit-console__title">Scene Slot Assignment</strong>
                  <span className="module-subtitle">
                    Persistence: {boardroomSlotAssignmentMode} / {boardroomSlotSaveStatus} — {boardroomSlotAssignmentMessage}
                  </span>
                  <span className="module-subtitle">
                    World surfaces: {worldSurfaceAssignmentMode} / {worldSurfaceSaveStatus} — {worldSurfaceAssignmentMessage}
                  </span>
                </div>
                <div className="monitor-config">
                  {BOARDROOM_SCENE_SLOT_IDS.map((slotId) => (
                    <label className="monitor-config__row" key={slotId}>
                      <span className="monitor-config__label">{slotId}</span>
                      <select
                        className="monitor-config__select"
                        value={boardroomSceneSlotAssignments[slotId]}
                        onChange={(event) => {
                          setBoardroomSceneSlotAssignments((current) => ({
                            ...current,
                            [slotId]: event.target.value,
                          }))
                        }}
                      >
                        {sceneSlotAssignmentOptions.map((option) => (
                          <option key={option.id} value={option.id}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                    </label>
                  ))}
                </div>
                <div className="boardroom-edit-console__lists">
                  <div>
                    <div className="module-subtitle">Section Focus</div>
                    <LineList
                      items={(bundle?.sections ?? []).slice(0, 8).map((section) => ({
                        label: section.title,
                        value: section.status,
                      }))}
                    />
                  </div>
                  <div>
                    <div className="module-subtitle">Panel Order</div>
                    <LineList
                      items={moduleOrder.map((moduleId, index) => ({
                        label: `${index + 1}. ${moduleTitles[moduleId]}`,
                        value: moduleId,
                      }))}
                    />
                  </div>
                </div>
              </div>
            </section>
          ) : null}
        </>
      ) : null}

      {viewMode === 'world' ? (
        <WorldRuntimeViewport
          active
          debug={editMode}
          zones={worldRuntimeZones}
          anchors={worldRuntimeAnchors}
          districtUrgencies={worldDistrictUrgencies}
          surfaceLayouts={worldSurfaceLayouts}
          presenceState={bundle?.agentPresenceState}
          onExit={() => {
            runSceneTransition('Returning To Boardroom', 'boardroom')
          }}
          onOpenPanel={(sourceZoneId) => {
            if (sourceZoneId) {
              setActiveSectionId(sourceZoneId)
              setPanelModeKey(sourceZoneId)
            }
            runSceneTransition('Opening Focused Panel', 'panel')
          }}
        />
      ) : null}

      {viewMode === 'panel' ? (
        <PanelWorkspace
          title={panelModeKey === 'settings' ? 'Settings' : titleForSectionOrPanel(panelModeKey ?? activeSectionId, bundle?.sections ?? [])}
          subtitle="Detailed modules for the active section. Open externally for multi-monitor command flow."
          modules={activePanelModules}
          activeModuleId={activePanelWorkstationId ? (workstationModuleById[activePanelWorkstationId] ?? null) : undefined}
          onActiveModuleChange={activePanelWorkstationId ? (moduleId) => setWorkstationActiveModule(activePanelWorkstationId, moduleId) : undefined}
          onOpenExternal={panelModeKey === 'settings' || panelModeKey === 'hermes_dashboard'
            ? undefined
            : () => openWorkstationWindow(getWorkstationManifestByZoneId(workstationManifests, panelModeKey ?? activeSectionId))}
          onBack={() => runSceneTransition('Returning To Boardroom', 'boardroom')}
        />
      ) : null}
      {viewMode === 'terminal' ? (
        <div style={{ height: '100vh' }}>
        </div>
      ) : null}
      <SceneTransitionOverlay active={transitionLabel !== null} label={transitionLabel ?? ''} />
    </div>
  )
}
