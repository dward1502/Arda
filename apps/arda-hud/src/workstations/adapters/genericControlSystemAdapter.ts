// sigil: REPAIR
import {
  createEmptyFleetViewModel,
  createEmptyWorkViewModel,
  isSafeActionDescriptor,
  sourceRef,
  type FleetLaneFitnessViewModel,
  type FleetLaneHeadroomViewModel,
  type FleetLaneOwnershipViewModel,
  type FleetProviderModel,
  type FleetProviderViewModel,
  type FleetViewModel,
  type WorkViewModel,
  type WorkstationActionDescriptor,
  type WorkstationMetric,
  type WorkstationSourceRef,
  type WorkstationStatus,
} from '../../scene/workstations/viewModels'

export type GenericHealthStatus = 'healthy' | 'degraded' | 'down' | 'unknown'
export type GenericTaskStatus = 'queued' | 'running' | 'blocked' | 'completed' | 'failed'
export type GenericSourceFreshness = 'fresh' | 'stale' | 'missing' | 'unknown'

export interface GenericControlSystemModelManifest {
  id: string
  context_window?: number | null
  healthy?: boolean
  default?: boolean
  capable_tasks?: string[]
}

export interface GenericControlSystemProviderManifest {
  id: string
  name?: string
  access_tier?: string
  quality_band?: string
  enabled?: boolean
  healthy?: boolean
  avg_latency_ms?: number | null
  active_connections?: number
  models?: GenericControlSystemModelManifest[]
  lane_headroom?: Partial<Record<'interactive' | 'execution' | 'background', number>>
}

export interface GenericControlSystemHealthCheckManifest {
  id: string
  label: string
  status: GenericHealthStatus
  checked_at?: string | null
  source_id?: string
}

export interface GenericControlSystemServiceManifest {
  id: string
  label: string
  provider_id?: string
  health?: GenericHealthStatus
}

export interface GenericControlSystemTaskManifest {
  id: string
  title: string
  status: GenericTaskStatus
  owner?: string
  source_id?: string
}

export interface GenericControlSystemDecisionManifest {
  id: string
  title: string
  status: 'pending' | 'approved' | 'rejected' | 'not_required'
}

export interface GenericControlSystemSourceManifest {
  id: string
  label: string
  path?: string
  freshness?: GenericSourceFreshness
  timestamp?: string | null
}

export interface GenericLaneRouteManifest {
  lane: string
  priority?: string
  provider_id: string
  model_id: string
  route_class?: string
  reason?: string
}

export interface GenericLaneFitnessManifest {
  lane: string
  provider_id: string
  avg_latency_ms?: number | null
  success_count?: number
  failure_count?: number
}

export interface GenericControlSystemManifest {
  schema_version: 'arda.generic_control_system.v1'
  system_id: string
  title: string
  generated_at?: string | null
  systems?: GenericControlSystemServiceManifest[]
  services?: GenericControlSystemServiceManifest[]
  providers?: GenericControlSystemProviderManifest[]
  health_checks?: GenericControlSystemHealthCheckManifest[]
  tasks?: GenericControlSystemTaskManifest[]
  jobs?: GenericControlSystemTaskManifest[]
  decisions?: GenericControlSystemDecisionManifest[]
  approvals?: GenericControlSystemDecisionManifest[]
  source_provenance?: GenericControlSystemSourceManifest[]
  action_descriptors?: WorkstationActionDescriptor[]
  lane_routes?: GenericLaneRouteManifest[]
  lane_fitness?: GenericLaneFitnessManifest[]
}

function metric(id: string, label: string, value: number | string, tone: WorkstationMetric['tone'] = 'neutral'): WorkstationMetric {
  return { id, label, value, tone }
}

function isGenericManifest(value: GenericControlSystemManifest | null | undefined): value is GenericControlSystemManifest {
  return value?.schema_version === 'arda.generic_control_system.v1'
}

function healthyStatus(status: GenericHealthStatus | undefined): boolean {
  return status === 'healthy' || status === undefined
}

function freshness(status: GenericSourceFreshness | undefined): GenericSourceFreshness {
  return status ?? 'unknown'
}

function sourceRefs(manifest: GenericControlSystemManifest): WorkstationSourceRef[] {
  const sources = manifest.source_provenance ?? []
  if (sources.length === 0) {
    return [sourceRef('generic_manifest', manifest.title, manifest.generated_at ? 'fresh' : 'unknown', manifest.generated_at ?? null)]
  }
  return sources.map((source) => sourceRef(
    source.id,
    source.label,
    freshness(source.freshness),
    source.timestamp ?? manifest.generated_at ?? null,
    source.path,
  ))
}

function safeActions(manifest: GenericControlSystemManifest): WorkstationActionDescriptor[] {
  return (manifest.action_descriptors ?? []).filter(isSafeActionDescriptor)
}

function providerModels(provider: GenericControlSystemProviderManifest): FleetProviderModel[] {
  return (provider.models ?? []).map((model) => ({
    id: model.id,
    contextWindow: typeof model.context_window === 'number' ? model.context_window : null,
    healthy: model.healthy ?? true,
    isDefault: model.default ?? false,
    capableTasks: model.capable_tasks ?? [],
  }))
}

function providers(manifest: GenericControlSystemManifest): FleetProviderViewModel[] {
  return (manifest.providers ?? []).map((provider) => ({
    providerId: provider.id,
    providerName: provider.name ?? provider.id,
    accessTier: provider.access_tier ?? 'generic',
    qualityBand: provider.quality_band ?? 'unknown',
    enabled: provider.enabled ?? true,
    healthy: provider.healthy ?? true,
    models: providerModels(provider),
    avgLatencyMs: typeof provider.avg_latency_ms === 'number' ? provider.avg_latency_ms : null,
    activeConnections: provider.active_connections ?? 0,
  }))
}

function laneOwnership(manifest: GenericControlSystemManifest): FleetLaneOwnershipViewModel[] {
  const routes = manifest.lane_routes ?? []
  if (routes.length === 0) {
    return ['interactive', 'execution', 'background'].map((lane) => ({
      lane,
      priority: lane,
      route: null,
    }))
  }
  return routes.map((route) => ({
    lane: route.lane,
    priority: route.priority ?? route.lane,
    route: {
      providerId: route.provider_id,
      modelId: route.model_id,
      routeClass: route.route_class ?? 'generic',
      reason: route.reason ?? '',
    },
  }))
}

function laneHeadroom(manifest: GenericControlSystemManifest, fleetProviders: FleetProviderViewModel[]): FleetLaneHeadroomViewModel[] {
  return fleetProviders.map((provider) => {
    const source = manifest.providers?.find((candidate) => candidate.id === provider.providerId)?.lane_headroom ?? {}
    return {
      providerId: provider.providerId,
      softCaps: {
        interactive: 0,
        execution: 0,
        background: 0,
      },
      laneHeadroom: {
        interactive: source.interactive ?? 0,
        execution: source.execution ?? 0,
        background: source.background ?? 0,
      },
    }
  })
}

function laneFitness(manifest: GenericControlSystemManifest): FleetLaneFitnessViewModel[] {
  return (manifest.lane_fitness ?? []).map((fitness) => ({
    lane: fitness.lane,
    providerId: fitness.provider_id,
    avgLatencyMs: typeof fitness.avg_latency_ms === 'number' ? fitness.avg_latency_ms : null,
    successCount: fitness.success_count ?? 0,
    failureCount: fitness.failure_count ?? 0,
  }))
}

function statusFromHealth(unhealthyCount: number, totalCount: number): WorkstationStatus {
  if (totalCount === 0) return 'empty'
  return unhealthyCount > 0 ? 'attention' : 'ok'
}

export function createGenericFleetViewModel(manifest: GenericControlSystemManifest | null | undefined): FleetViewModel {
  if (!isGenericManifest(manifest)) {
    return createEmptyFleetViewModel(['Generic control-system manifest unavailable.'])
  }

  const checks = manifest.health_checks ?? []
  const services = [...(manifest.systems ?? []), ...(manifest.services ?? [])]
  const unhealthyChecks = checks.filter((check) => check.status === 'degraded' || check.status === 'down').length
  const unhealthyServices = services.filter((service) => !healthyStatus(service.health)).length
  const fleetProviders = providers(manifest)
  const healthyProviders = fleetProviders.filter((provider) => provider.enabled && provider.healthy).length

  return {
    roleId: 'fleet',
    title: `${manifest.title} Fleet`,
    status: statusFromHealth(unhealthyChecks + unhealthyServices, checks.length + services.length),
    summary: [
      `${services.length} systems/services registered`,
      `${healthyProviders}/${fleetProviders.length} providers healthy`,
      `${unhealthyChecks + unhealthyServices} health checks need attention`,
    ],
    metrics: [
      metric('systems_services', 'Systems + Services', services.length, services.length > 0 ? 'good' : 'attention'),
      metric('providers', 'Providers', fleetProviders.length, fleetProviders.length > 0 ? 'good' : 'attention'),
      metric('healthy_providers', 'Healthy Providers', healthyProviders, healthyProviders > 0 ? 'good' : 'attention'),
      metric('health_attention', 'Health Attention', unhealthyChecks + unhealthyServices, unhealthyChecks + unhealthyServices > 0 ? 'attention' : 'good'),
    ],
    sources: sourceRefs(manifest),
    actions: safeActions(manifest),
    providers: fleetProviders,
    laneOwnership: laneOwnership(manifest),
    laneHeadroom: laneHeadroom(manifest, fleetProviders),
    laneFitness: laneFitness(manifest),
  }
}

export function createGenericWorkViewModel(manifest: GenericControlSystemManifest | null | undefined): WorkViewModel {
  if (!isGenericManifest(manifest)) {
    return createEmptyWorkViewModel()
  }

  const workItems = [...(manifest.tasks ?? []), ...(manifest.jobs ?? [])]
  const active = workItems.filter((item) => item.status === 'queued' || item.status === 'running').length
  const blocked = workItems.filter((item) => item.status === 'blocked' || item.status === 'failed').length
  const decisions = [...(manifest.decisions ?? []), ...(manifest.approvals ?? [])]
  const pendingDecisions = decisions.filter((decision) => decision.status === 'pending').length

  return {
    roleId: 'work',
    title: `${manifest.title} Work`,
    status: workItems.length === 0 ? 'empty' : blocked > 0 || pendingDecisions > 0 ? 'attention' : 'ok',
    summary: [
      `${active}/${workItems.length} tasks or jobs active`,
      `${blocked} blocked or failed work items`,
      `${pendingDecisions} decisions pending`,
    ],
    metrics: [
      metric('work_items', 'Tasks + Jobs', workItems.length, workItems.length > 0 ? 'good' : 'attention'),
      metric('active_work', 'Active Work', active, active > 0 ? 'good' : 'neutral'),
      metric('blocked_work', 'Blocked Work', blocked, blocked > 0 ? 'attention' : 'good'),
      metric('pending_decisions', 'Pending Decisions', pendingDecisions, pendingDecisions > 0 ? 'attention' : 'good'),
    ],
    sources: sourceRefs(manifest),
    actions: safeActions(manifest),
  }
}
