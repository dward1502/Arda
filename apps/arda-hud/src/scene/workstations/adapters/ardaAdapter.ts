// sigil: REPAIR
import type {
  ArdaBundle,
  JsonRecord,
} from '../../../lib/ardaSource'
import {
  createEmptyFleetViewModel,
  sourceRef,
  type FleetLaneFitnessViewModel,
  type FleetLaneHeadroomViewModel,
  type FleetLaneOwnershipViewModel,
  type FleetNodeViewModel,
  type FleetProviderModel,
  type FleetProviderTier,
  type FleetProviderViewModel,
  type FleetViewModel,
  type RoutingCommunicationPathway,
  type RoutingLaneViewModel,
  type RoutingProviderViewModel,
  type RoutingViewModel,
  type WorkstationMetric,
} from '../viewModels'

export interface ArdaFleetTargetSummary {
  displayName: string
  providerId: string
}

export interface ArdaFleetHealth {
  totalTargets: number
  liveTargets: number
  routableProviders: number
  intentionalOffline: number
  unexpectedOffline: number
  intentionalOfflineTargets: ArdaFleetTargetSummary[]
  unexpectedOfflineTargets: ArdaFleetTargetSummary[]
}

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as JsonRecord
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function getString(value: unknown, fallback = ''): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

function getNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function getBoolean(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback
}

function nullableNumber(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function getOperatorRuntimeSurface(bundle: ArdaBundle): JsonRecord | null {
  return asRecord(bundle.operatorRuntimeStatus)
}

function getCharonProviderRecords(bundle: ArdaBundle): JsonRecord[] {
  const pressure = asRecord(bundle.manweRouter?.provider_pressure)
  return [
    ...asArray(pressure?.providers),
    ...(pressure?.local_fallback ? [pressure.local_fallback] : []),
  ]
    .map((provider) => asRecord(provider))
    .filter((provider): provider is JsonRecord => provider !== null)
}

function getRoutableProviderModels(provider: JsonRecord): FleetProviderModel[] {
  return asArray(provider.models)
    .map((model) => {
      const modelRecord = asRecord(model)
      if (!modelRecord) {
        const id = getString(model)
        return id ? {
          id,
          contextWindow: null,
          healthy: true,
          isDefault: false,
          capableTasks: [],
        } : null
      }
      return {
        id: getString(modelRecord.id, 'unknown'),
        contextWindow: nullableNumber(modelRecord.context_window),
        healthy: getBoolean(modelRecord.healthy, true),
        isDefault: getBoolean(modelRecord.is_default, false),
        capableTasks: asArray(modelRecord.capable_tasks).map((task) => getString(task)).filter(Boolean),
      }
    })
    .filter((model): model is FleetProviderModel => model !== null)
}

function getRoutableProviders(bundle: ArdaBundle): FleetProviderViewModel[] {
  const charonProviders = getCharonProviderRecords(bundle)
  if (charonProviders.length > 0) {
    return charonProviders.map((provider) => ({
      providerId: getString(provider.id, 'unknown'),
      providerName: getString(provider.name, getString(provider.id, 'unknown')),
      accessTier: provider.access_tier as FleetProviderTier,
      qualityBand: getString(provider.quality_band, 'unknown'),
      enabled: getBoolean(provider.enabled, false),
      healthy: getBoolean(provider.healthy, false),
      models: getRoutableProviderModels(provider),
      avgLatencyMs: nullableNumber(provider.avg_latency_ms),
      activeConnections: getNumber(provider.active_connections, 0),
    }))
  }

  const operator = getOperatorRuntimeSurface(bundle)
  return asArray(operator?.routable_providers)
    .map((provider) => asRecord(provider))
    .filter((provider): provider is JsonRecord => provider !== null)
    .map((provider) => {
      const providerId = getString(provider.provider_id, 'unknown')
      return {
        providerId,
        providerName: providerId,
        accessTier: 'operator_projection',
        qualityBand: 'unknown',
        enabled: true,
        healthy: true,
        models: getRoutableProviderModels(provider),
        avgLatencyMs: nullableNumber(provider.avg_latency_ms),
        activeConnections: getNumber(provider.active_connections, 0),
      }
    })
}

function getLaneOwnership(bundle: ArdaBundle): FleetLaneOwnershipViewModel[] {
  const operator = getOperatorRuntimeSurface(bundle)
  const laneRoutes = asRecord(operator?.lane_routes)
  const labels: Record<string, string> = {
    interactive: 'Normal Chat',
    execution: 'High Code',
    background: 'Low Background',
  }

  return ['interactive', 'execution', 'background'].map((lane) => {
    const route = asRecord(laneRoutes?.[lane])
    return {
      lane,
      priority: labels[lane] ?? lane,
      route: route ? {
        providerId: getString(route.provider_id, 'unknown'),
        modelId: getString(route.model_id, 'unknown'),
        routeClass: getString(route.route_class, 'unknown'),
        reason: getString(route.reason, ''),
      } : null,
    }
  })
}

function getLaneHeadroom(bundle: ArdaBundle, providers: FleetProviderViewModel[]): FleetLaneHeadroomViewModel[] {
  const operator = getOperatorRuntimeSurface(bundle)
  const laneHeadroom = asRecord(operator?.lane_headroom)
  return providers.map((provider) => {
    const softCaps = asRecord(asArray(operator?.routable_providers)
      .map((entry) => asRecord(entry))
      .find((entry) => getString(entry?.provider_id) === provider.providerId)?.soft_caps)
    return {
      providerId: provider.providerId,
      softCaps: {
        interactive: getNumber(softCaps?.interactive, 0),
        execution: getNumber(softCaps?.execution, 0),
        background: getNumber(softCaps?.background, 0),
      },
      laneHeadroom: {
        interactive: getNumber(asRecord(laneHeadroom?.interactive)?.[provider.providerId], 0),
        execution: getNumber(asRecord(laneHeadroom?.execution)?.[provider.providerId], 0),
        background: getNumber(asRecord(laneHeadroom?.background)?.[provider.providerId], 0),
      },
    }
  })
}

function getLaneFitness(bundle: ArdaBundle): FleetLaneFitnessViewModel[] {
  const operator = getOperatorRuntimeSurface(bundle)
  const laneFitness = asRecord(operator?.lane_fitness)
  return Object.entries(laneFitness ?? {}).flatMap(([lane, providers]) => {
    const providerMap = asRecord(providers)
    return Object.entries(providerMap ?? {}).map(([providerId, state]) => {
      const record = asRecord(state)
      return {
        lane,
        providerId,
        avgLatencyMs: record ? nullableNumber(record.avg_latency_ms) : null,
        successCount: record ? getNumber(record.success_count, 0) : 0,
        failureCount: record ? getNumber(record.failure_count, 0) : 0,
      }
    })
  })
}

function getOfflineTargets(operator: JsonRecord | null, key: string): ArdaFleetTargetSummary[] {
  return asArray(operator?.[key])
    .map((target) => asRecord(target))
    .filter((target): target is JsonRecord => target !== null)
    .map((target) => ({
      displayName: getString(target.display_name, getString(target.target_id, 'unknown')),
      providerId: getString(target.provider_id, 'unknown'),
    }))
}

export function createArdaFleetHealth(bundle: ArdaBundle): ArdaFleetHealth {
  const operator = getOperatorRuntimeSurface(bundle)
  const summary = asRecord(operator?.summary)
  const fleet = asRecord(operator?.fleet)
  const intentionalOfflineTargets = getOfflineTargets(operator, 'intentional_offline_targets')
  const unexpectedOfflineTargets = getOfflineTargets(operator, 'unexpected_offline_targets')

  return {
    totalTargets: getNumber(fleet?.targets_total, 0),
    liveTargets: getNumber(summary?.fleet_live_llm_nodes_total, 0),
    routableProviders: getNumber(summary?.fleet_routable_local_providers_total, 0),
    intentionalOffline: intentionalOfflineTargets.length,
    unexpectedOffline: getNumber(summary?.unexpected_offline_total, unexpectedOfflineTargets.length),
    intentionalOfflineTargets,
    unexpectedOfflineTargets,
  }
}

function metric(id: string, label: string, value: number, tone: WorkstationMetric['tone'] = 'neutral'): WorkstationMetric {
  return { id, label, value, tone }
}

const FLEET_SOURCES = [
  ['fleetRuntime', 'fleet_runtime', 'Fleet Runtime', 'core/state/fleet_runtime.json'],
  ['fleetNodes', 'fleet_nodes', 'Fleet Nodes', 'core/state/fleet_nodes.json'],
  ['fleetModels', 'fleet_models', 'Fleet Models', 'core/state/fleet_models.json'],
  ['fleetHealth', 'fleet_health', 'Fleet Health', 'core/state/fleet_health.json'],
  ['fleetHardware', 'fleet_hardware', 'Fleet Hardware', 'core/state/fleet_hardware.json'],
  ['fleetBackbone', 'fleet_backbone', 'Fleet Backbone', 'core/state/fleet_backbone.json'],
] as const

function fleetSourceRefs(bundle: ArdaBundle) {
  const bundleTime = Date.parse(bundle.generatedAt)
  return FLEET_SOURCES.map(([key, id, label, path]) => {
    const record = asRecord(bundle[key])
    const timestamp = getString(record?.generated_at_utc) || null
    const ageMs = timestamp ? bundleTime - Date.parse(timestamp) : Number.NaN
    const freshness = !record ? 'missing' : Number.isFinite(ageMs) && ageMs <= 300_000 ? 'fresh' : timestamp ? 'stale' : 'unknown'
    return sourceRef(id, label, freshness, timestamp, path, `${label} ${freshness}: ${path}.`)
  })
}

function getFleetNodes(bundle: ArdaBundle): FleetNodeViewModel[] {
  return asArray(bundle.fleetNodes?.nodes)
    .map(asRecord)
    .filter((node): node is JsonRecord => node !== null)
    .map((node) => {
      const configured = asRecord(node.configured)
      const observed = asRecord(node.observed)
      const hardware = asRecord(observed?.hardware)
      const memory = asRecord(hardware?.memory)
      const gpuCount = asArray(hardware?.nvidia_gpus).length || asArray(hardware?.gpu_inventory).length
      return {
        id: getString(configured?.id, getString(node.id, 'unknown')),
        displayName: getString(node.display_name, getString(configured?.display_name, 'Unknown node')),
        hostname: getString(configured?.hostname, getString(observed?.hostname, 'unknown')),
        nodeClass: getString(configured?.node_class, 'unknown'),
        online: getBoolean(observed?.online, false),
        enrollmentStatus: getString(configured?.enrollment_status, 'unknown'),
        expectedModels: asArray(configured?.expected_models).map((model) => getString(model)).filter(Boolean),
        hardwareSummary: hardware ? `${getString(memory?.total, 'memory unknown')} · ${gpuCount} GPU${gpuCount === 1 ? '' : 's'}` : 'hardware unavailable',
      }
    })
}

export function createArdaFleetViewModel(bundle: ArdaBundle): FleetViewModel {
  const operator = getOperatorRuntimeSurface(bundle)
  if (!operator) {
    return createEmptyFleetViewModel()
  }

  const summary = asRecord(operator.summary)
  const fleet = asRecord(operator.fleet)
  const fleetNodeCounts = asRecord(bundle.fleetNodes?.counts)
  const totalTargets = getNumber(fleetNodeCounts?.configured_total, getNumber(fleet?.targets_total, 0))
  const liveTargets = getNumber(fleetNodeCounts?.online_total, getNumber(summary?.fleet_live_llm_nodes_total, 0))
  const routableProviderCount = getNumber(summary?.fleet_routable_local_providers_total, 0)
  const unexpectedOffline = getNumber(summary?.unexpected_offline_total, 0)
  const providers = getRoutableProviders(bundle)

  return {
    roleId: 'fleet',
    title: 'Fleet',
    status: unexpectedOffline > 0 ? 'attention' : 'ok',
    summary: [
      `${liveTargets}/${totalTargets} fleet targets live`,
      `${providers.length} routable provider${providers.length === 1 ? '' : 's'} available`,
      unexpectedOffline > 0 ? `${unexpectedOffline} unexpected offline target${unexpectedOffline === 1 ? '' : 's'}` : 'No unexpected offline targets',
    ],
    metrics: [
      metric('total_targets', 'Total Targets', totalTargets),
      metric('live_targets', 'Live Targets', liveTargets, liveTargets > 0 ? 'good' : 'attention'),
      metric('routable_providers', 'Routable Providers', routableProviderCount || providers.length, providers.length > 0 ? 'good' : 'attention'),
      metric('unexpected_offline', 'Unexpected Offline', unexpectedOffline, unexpectedOffline > 0 ? 'attention' : 'good'),
    ],
    sources: [
      sourceRef('operator_runtime_status', 'Operator Runtime Status', 'fresh', bundle.generatedAt, 'core/state/operator_runtime_status.json'),
      sourceRef('charon_router', 'Charon Router', bundle.manweRouter ? 'fresh' : 'missing', bundle.generatedAt, 'core/state/manwe_router.json'),
      ...fleetSourceRefs(bundle),
    ],
    actions: [
      {
        id: 'refresh_fleet_projection',
        label: 'Refresh fleet projection',
        safety: 'read_only',
        description: 'Reload ARDA source projections before making routing claims.',
      },
    ],
    previewKinds: [],
    focusedCapabilities: [],
    rawDisclosure: false,
    providers,
    laneOwnership: getLaneOwnership(bundle),
    laneHeadroom: getLaneHeadroom(bundle, providers),
    laneFitness: getLaneFitness(bundle),
    nodes: getFleetNodes(bundle),
    backboneNodeId: getString(asRecord(bundle.fleetBackbone?.backbone_node)?.id) || null,
  }
}

function routingSourceRef(bundle: ArdaBundle, value: JsonRecord | null, id: string, label: string, path: string) {
  if (!value) return sourceRef(id, label, 'missing', null, path, `${label} missing: ${path}.`)
  const timestamp = getString(value.generated_at_utc) || getString(value.generated_at) || null
  const ageMs = timestamp ? Date.parse(bundle.generatedAt) - Date.parse(timestamp) : Number.NaN
  const status = timestamp && Number.isFinite(ageMs) && ageMs > 300_000 ? 'stale' : timestamp ? 'snapshot' : 'projected'
  return sourceRef(id, label, status, timestamp, path, `${label} ${status}: ${path}.`)
}

function getRoutingProviders(bundle: ArdaBundle): RoutingProviderViewModel[] {
  const intelligence = asRecord(bundle.providerIntelligence?.providers)
  const providers = new Map(getRoutableProviders(bundle).map((provider) => [provider.providerId, provider]))
  for (const [providerId, value] of Object.entries(intelligence ?? {})) {
    if (providers.has(providerId)) continue
    const projected = asRecord(value)
    providers.set(providerId, {
      providerId,
      providerName: getString(projected?.name, providerId),
      enabled: getBoolean(projected?.enabled, false),
      healthy: getBoolean(projected?.healthy, false),
      activeConnections: 0,
      models: asArray(projected?.models).map((model) => {
        const record = asRecord(model)
        return {
          id: getString(record?.id, 'unknown'),
          contextWindow: nullableNumber(record?.context_window),
          healthy: getBoolean(record?.healthy, getBoolean(projected?.healthy, false)),
          isDefault: getBoolean(record?.is_default, false),
          capableTasks: asArray(record?.capable_tasks).map((task) => getString(task)).filter(Boolean),
        }
      }),
      accessTier: getString(projected?.access_tier).includes('local') ? 'local' : 'cloud',
      qualityBand: getString(projected?.quality_band, 'unknown'),
      avgLatencyMs: null,
    })
  }
  return [...providers.values()].map((provider) => {
    const projected = asRecord(intelligence?.[provider.providerId])
    return {
      providerId: provider.providerId,
      providerName: getString(projected?.name, provider.providerName),
      enabled: projected ? getBoolean(projected.enabled, provider.enabled) : provider.enabled,
      healthy: projected ? getBoolean(projected.healthy, provider.healthy) : provider.healthy,
      activeConnections: provider.activeConnections,
      modelCount: projected ? asArray(projected.models).length : provider.models.length,
      accessTier: getString(projected?.access_tier, provider.accessTier),
      qualityBand: getString(projected?.quality_band, provider.qualityBand),
    }
  })
}

function getRoutingLanes(bundle: ArdaBundle): RoutingLaneViewModel[] {
  const operator = getOperatorRuntimeSurface(bundle)
  const ownership = getLaneOwnership(bundle)
  const headroom = asRecord(operator?.lane_headroom)
  const providerRecords = asArray(operator?.routable_providers).map(asRecord).filter((record): record is JsonRecord => record !== null)
  const fitness = getLaneFitness(bundle)
  return ownership.filter((entry) => entry.route !== null).map((entry) => {
    const route = entry.route!
    const provider = providerRecords.find((record) => getString(record.provider_id) === route.providerId)
    const caps = asRecord(provider?.soft_caps)
    const laneFitness = fitness.find((record) => record.lane === entry.lane && record.providerId === route.providerId)
    return {
      lane: entry.lane,
      label: entry.priority,
      providerId: route.providerId,
      modelId: route.modelId,
      routeClass: route.routeClass,
      reason: route.reason,
      headroom: nullableNumber(asRecord(headroom?.[entry.lane])?.[route.providerId]),
      softCap: nullableNumber(caps?.[entry.lane]),
      avgLatencyMs: laneFitness?.avgLatencyMs ?? null,
      successes: laneFitness?.successCount ?? 0,
      failures: laneFitness?.failureCount ?? 0,
    }
  })
}

function getCommunicationPathways(bundle: ArdaBundle): RoutingCommunicationPathway[] {
  const records = [...(bundle.hermesMessages ?? []), ...(bundle.hermesAgentGatewayReceipts ?? [])]
  const pathways = new Map<string, RoutingCommunicationPathway>()
  for (const entry of records) {
    const record = asRecord(entry)
    const id = getString(record?.platform, getString(record?.channel, getString(record?.ingress)))
    if (!id) continue
    const existing = pathways.get(id)
    pathways.set(id, {
      id,
      label: id.charAt(0).toUpperCase() + id.slice(1),
      state: getString(record?.status, existing?.state ?? 'observed'),
      receipts: (existing?.receipts ?? 0) + 1,
    })
  }
  return [...pathways.values()].sort((a, b) => a.id.localeCompare(b.id))
}

export function createArdaRoutingViewModel(bundle: ArdaBundle): RoutingViewModel {
  const operator = getOperatorRuntimeSurface(bundle)
  const charon = asRecord(operator?.charon)
  const pressure = asRecord(charon?.budget_pressure)
  const providers = getRoutingProviders(bundle)
  const lanes = getRoutingLanes(bundle)
  const successes = getNumber(charon?.recent_route_successes, 0)
  const failures = getNumber(charon?.recent_route_failures, 0)
  const healthyProviders = getNumber(charon?.providers_healthy, providers.filter((provider) => provider.healthy).length)
  const enabledProviders = getNumber(charon?.providers_enabled, providers.filter((provider) => provider.enabled).length)
  const highestLevel = getString(pressure?.highest_level, 'unavailable')
  const communicationPathways = getCommunicationPathways(bundle)

  return {
    roleId: 'routing',
    title: 'Routing + Communications',
    status: !operator ? 'empty' : failures > 0 || highestLevel === 'warning' || highestLevel === 'critical' ? 'attention' : 'ok',
    summary: !operator ? ['Routing projection unavailable.'] : [
      `${healthyProviders}/${enabledProviders} enabled providers healthy`,
      `${successes} recent route successes · ${failures} failures`,
      `${communicationPathways.length} communication pathway${communicationPathways.length === 1 ? '' : 's'} evidenced`,
    ],
    metrics: [
      metric('healthy_providers', 'Healthy Providers', healthyProviders, healthyProviders > 0 ? 'good' : 'attention'),
      metric('enabled_providers', 'Enabled Providers', enabledProviders),
      metric('route_successes', 'Route Successes', successes, 'good'),
      metric('route_failures', 'Route Failures', failures, failures > 0 ? 'attention' : 'good'),
    ],
    sources: [
      routingSourceRef(bundle, operator, 'operator_runtime_status', 'Operator Runtime', 'core/state/operator_runtime_status.json'),
      routingSourceRef(bundle, asRecord(bundle.manweRouter), 'manwe_router', 'CHARON Router', 'core/state/manwe_router.json'),
      routingSourceRef(bundle, asRecord(bundle.providerIntelligence), 'provider_intelligence', 'Provider Intelligence', 'core/state/provider_intelligence.json'),
      routingSourceRef(bundle, asRecord(bundle.providerTokenUsage), 'provider_token_usage', 'Provider Token Usage', 'core/state/provider_token_usage.json'),
      routingSourceRef(bundle, asRecord(bundle.chronosRuntime), 'chronos_runtime', 'CHRONOS Runtime', 'core/state/chronos_runtime.json'),
      sourceRef('hermes_communications', 'Hermes Communications', communicationPathways.length > 0 ? 'projected' : 'unavailable', bundle.generatedAt, undefined, communicationPathways.length > 0 ? 'Hermes message and gateway receipt projections.' : 'No Hermes communication receipts loaded.'),
    ],
    actions: [
      { id: 'arda.chronos_run_provider_checks', label: 'Run Provider Checks', safety: 'read_only', command: 'run_chronos_provider_checks' },
      { id: 'charon.refresh_provider_intelligence', label: 'Refresh Provider Intelligence', safety: 'read_only', command: 'run_charon_provider_intelligence_refresh' },
    ],
    previewKinds: ['route_flow', 'lane_pressure', 'communication_receipts'],
    focusedCapabilities: ['lane_ownership', 'provider_detail', 'route_fitness', 'budget_pressure', 'read_only_refresh_actions'],
    rawDisclosure: false,
    providers,
    lanes,
    routeHistory: { successes, failures },
    budgetPressure: {
      highestLevel,
      cooldownTotal: getNumber(pressure?.cooldown_total, 0),
      exhaustedTotal: getNumber(pressure?.exhausted_total, 0),
    },
    communicationPathways,
    provenanceClause: 'Routing claims are derived from loaded CHARON/operator snapshots; Service Health remains Fleet-owned.',
  }
}
