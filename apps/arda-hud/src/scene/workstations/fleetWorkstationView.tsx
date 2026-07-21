import type {FleetViewModel } from '../workstations/viewModels'
export interface FloatingWorkstationState {
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

export const FLOATING_WORKSTATION_BASE_Z_INDEX = 320
const FLOATING_WORKSTATION_MARGIN = 28
const FLOATING_WORKSTATION_TILE_GAP = 18

export function clampFloatingWorkstationValue(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(value, max))
}

function getFloatingWorkstationViewport() {
  if (typeof window === 'undefined') {
    return { width: 1440, height: 900 }
  }
  return {
    width: window.innerWidth,
    height: window.innerHeight,
  }
}

export function getFloatingWorkstationTileLayout(index: number, total: number) {
  const viewport = getFloatingWorkstationViewport()
  const safeTotal = Math.max(1, total)
  const margin = FLOATING_WORKSTATION_MARGIN
  const gap = FLOATING_WORKSTATION_TILE_GAP
  const availableWidth = Math.max(360, viewport.width - margin * 2)
  const availableHeight = Math.max(280, viewport.height - margin * 2)

  if (safeTotal === 1) {
    const width = Math.min(940, availableWidth)
    const height = Math.min(680, availableHeight)
    return {
      x: Math.round(margin + (availableWidth - width) / 2),
      y: Math.round(margin + Math.max(0, (availableHeight - height) * 0.28)),
      width,
      height,
    }
  }

  const columns = safeTotal <= 4 ? 2 : Math.min(3, Math.ceil(Math.sqrt(safeTotal)))
  const rows = Math.ceil(safeTotal / columns)
  const tileWidth = Math.floor((availableWidth - gap * (columns - 1)) / columns)
  const tileHeight = Math.floor((availableHeight - gap * (rows - 1)) / rows)
  const row = Math.floor(index / columns)
  const column = index % columns
  const rowItemCount = Math.min(columns, safeTotal - row * columns)
  const rowWidth = rowItemCount * tileWidth + Math.max(0, rowItemCount - 1) * gap
  const rowOffset = Math.max(0, (availableWidth - rowWidth) / 2)

  return {
    x: Math.round(margin + rowOffset + column * (tileWidth + gap)),
    y: Math.round(margin + row * (tileHeight + gap)),
    width: clampFloatingWorkstationValue(tileWidth, 320, availableWidth),
    height: clampFloatingWorkstationValue(tileHeight, 240, availableHeight),
  }
}

export function getFloatingWorkstationCenteredLayout() {
  const viewport = getFloatingWorkstationViewport()
  const margin = FLOATING_WORKSTATION_MARGIN
  const availableWidth = Math.max(360, viewport.width - margin * 2)
  const availableHeight = Math.max(280, viewport.height - margin * 2)
  const width = Math.min(940, availableWidth)
  const height = Math.min(680, availableHeight)

  return {
    x: Math.round(margin + Math.max(0, (availableWidth - width) / 2)),
    y: Math.round(margin + Math.max(0, (availableHeight - height) / 2)),
    width,
    height,
  }
}

export function FleetFocusedWorkstationView({ fleetViewModel }: { fleetViewModel: FleetViewModel | null }) {
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