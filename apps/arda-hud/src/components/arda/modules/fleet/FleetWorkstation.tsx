// sigil: REPAIR
import { useMemo } from 'react'

export interface FleetFocusedWorkstationViewProps {
  fleetViewModel: {
    providers: Array<{ id: string; enabled: boolean; healthy: boolean }>
    metrics: Array<{ id: string }>
    knownProviders: string[]
    laneOwnership: Array<{ lane: string; priority: string; route?: { providerId: string; modelId: string; routeClass: string; reason: string } | null }>
    laneFitness: Array<{ lane: string; providerId: string; avgLatencyMs: number | null; successCount: number; failureCount: number }>
    fleetHealth: {
      totalTargets: number
      liveTargets: number
      routableProviders: number
      intentionalOffline: number
      unexpectedOffline: number
      intentionalOfflineTargets: Array<{ displayName: string; providerId: string }>
      unexpectedOfflineTargets: Array<{ displayName: string; providerId: string }>
    }
    runtimeDrift: {
      totalNodes: number
      driftedNodes: number
      items: Array<{ nodeId: string; displayName: string; providerId: string; declaredModel: string; declaredContextWindow: number | null; charonContextWindow: number | null; actualProcessContextWindow: number | null; declaredVsCharon: boolean; declaredVsLocalProcess: boolean; localRuntimeStatus: string }>
    }
    operatorCockpit: { queue: { openTotal: number; items: Array<{ id: string; title: string; owner: string; status: string; priority: string }> }; warden: { effectiveAttention: number } }
    actionDescriptors: Array<{ id: string; label: string; purpose: string; riskLevel: string }>
    capabilityStatuses: Array<{ id: string; manualRunAvailable: boolean }>
  } | null
}

export function buildSafeFleetViewModel(viewModel: FleetFocusedWorkstationViewProps['fleetViewModel']) {
  if (!viewModel) return viewModel

  const providers = viewModel.providers ?? []
  const primaryProvider = providers.find((provider) => provider.enabled && provider.healthy) ?? providers[0] ?? null
  const offlineMetric = (viewModel.metrics ?? []).find((metric) => metric.id === 'unexpected_offline')

  const knownProviders = viewModel.knownProviders ?? (primaryProvider ? [primaryProvider.id] : [])
  const providerIdSet = new Set(knownProviders)
  providers.forEach((provider) => {
    if (provider.id && !providerIdSet.has(provider.id)) {
      knownProviders.push(provider.id)
      providerIdSet.add(provider.id)
    }
  })

  const laneOwnership = (viewModel.laneOwnership ?? []).map((entry) => ({
    ...entry,
    route: entry.route ?? {
      providerId: knownProviders[0] ?? 'unassigned',
      modelId: 'unset',
      routeClass: 'unavailable',
      reason: 'no route data loaded yet',
    },
  }))

  return { ...viewModel, providers, knownProviders, laneOwnership }
}

export function buildWorkstationModuleRole(moduleId: string, viewModel: unknown): { moduleId: string; title: string; node: React.ReactNode } | null {
  if (moduleId !== 'FLEET_FOCUSED') return null
  const accepted = viewModel as FleetFocusedWorkstationViewProps['fleetViewModel']
  return {
    moduleId,
    title: 'Fleet Systems',
    node: <FleetFocusedWorkstationView fleetViewModel={accepted} />,
  }
}

export default function FleetFocusedWorkstationView({ fleetViewModel }: FleetFocusedWorkstationViewProps) {
  const safeModel = useMemo(() => buildSafeFleetViewModel(fleetViewModel), [fleetViewModel])

  if (!safeModel) {
    return (
      <section className="src/components/arda/modules/fleet/FleetWorkstation.tsx fleet-focused-view fleet-focused-view--empty">
        <span className="fleet-focused-view__eyebrow">Fleet View Model</span>
        <h3>Fleet projection unavailable</h3>
        <p>Waiting for operator runtime and Charon router projections.</p>
      </section>
    )
  }

  return (
    <section className="fleet-focused-view fleet-focused-view--loaded">
      <div className="fleet-focused-view__hero">
        <div>
          <span className="fleet-focused-view__eyebrow">Fleet View Model</span>
          <h3>Fleet Systems</h3>
        </div>
        <span className="fleet-focused-view__status">projected</span>
      </div>
      <section className="surface-card">
        <h3>Context Health</h3>
        <ul>
          <li>Primary provider - {safeModel.providers.find((provider) => provider.enabled && provider.healthy)?.id ?? 'not available'}</li>
          <li>Known providers - {safeModel.knownProviders.join(', ') || 'none'}</li>
        </ul>
      </section>
      <section className="surface-card">
        <h3>Fleet Recovery</h3>
        <ul>
          <li>Live targets - {safeModel.providers.filter((provider) => provider.enabled && provider.healthy).length}/{safeModel.providers.length}</li>
          <li>Routable providers - {safeModel.knownProviders.length}</li>
          <li>Unexpected offline - {(safeModel.metrics ?? []).filter((metric) => metric.id === 'unexpected_offline').length}</li>
        </ul>
      </section>
      <section className="surface-card">
        <h3>Fleet Health</h3>
        <ul>
          <li>totalTargets - {safeModel.fleetHealth.totalTargets}</li>
          <li>liveTargets - {safeModel.fleetHealth.liveTargets}</li>
          <li>routableProviders - {safeModel.fleetHealth.routableProviders}</li>
          <li>intentionalOffline - {safeModel.fleetHealth.intentionalOffline}</li>
          <li>unexpectedOffline - {safeModel.fleetHealth.unexpectedOffline}</li>
        </ul>
      </section>
      <section className="surface-card">
        <h3>Lane Ownership</h3>
        <ul>
          {safeModel.laneOwnership.map((entry) => (
            <li key={entry.lane}>{entry.lane} / {entry.priority} / {entry.route?.providerId ?? '-'}</li>
          ))}
        </ul>
      </section>
      <section className="surface-card">
        <h3>Lane Fitness</h3>
        <ul>
          {safeModel.laneFitness.map((entry) => (
            <li key={`${entry.lane}-${entry.providerId}`}>{entry.lane} - {entry.providerId} - {entry.avgLatencyMs ?? 'n/a'} ms</li>
          ))}
        </ul>
      </section>
      <section className="surface-card">
        <h3>Runtime Drift</h3>
        <ul>
          <li>{safeModel.runtimeDrift.totalNodes} nodes / {safeModel.runtimeDrift.driftedNodes} drifted</li>
          {safeModel.runtimeDrift.items.slice(0, 4).map((item) => (
            <li key={item.nodeId}>{item.displayName} / {item.providerId} / drift {String(item.declaredVsCharon || item.declaredVsLocalProcess)}</li>
          ))}
        </ul>
      </section>
      <section className="surface-card">
        <h3>Operator Cockpit</h3>
        <ul>
          <li>queue openTotal - {safeModel.operatorCockpit.queue.openTotal}</li>
          <li>warden effectiveAttention - {safeModel.operatorCockpit.warden.effectiveAttention}</li>
        </ul>
      </section>
      <section className="surface-card">
        <h3>Action Contracts</h3>
        <ul>
          {safeModel.actionDescriptors.map((action) => (
            <li key={action.id}>{action.label} / {action.riskLevel}</li>
          ))}
          {safeModel.actionDescriptors.length === 0 ? <li>No action descriptors available.</li> : null}
        </ul>
      </section>
    </section>
  )
}
