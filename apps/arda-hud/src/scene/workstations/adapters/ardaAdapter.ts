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
