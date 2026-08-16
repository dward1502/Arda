import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import type { FleetViewModel } from '../workstations/viewModels'
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

export function FleetFocusedWorkstationView({ fleetViewModel, onRefresh }: { fleetViewModel: FleetViewModel | null; onRefresh?: () => void }) {
  const nodes = fleetViewModel?.nodes ?? []
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(nodes[0]?.id ?? null)
  useEffect(() => {
    if (!nodes.some((node) => node.id === selectedNodeId)) {
      setSelectedNodeId(nodes[0]?.id ?? null)
    }
  }, [nodes, selectedNodeId])

  if (!fleetViewModel) {
    return (
      <div className="fleet-focused-view fleet-focused-view--empty">
        <span className="fleet-focused-view__eyebrow">Fleet View Model</span>
        <h3>Fleet projection unavailable</h3>
        <p>Fleet projection consumer is unavailable.</p>
      </div>
    )
  }

  const selectedNode = nodes.find((node) => node.id === selectedNodeId) ?? null
  const offlineMetric = fleetViewModel.metrics.find((metric) => metric.id === 'unexpected_offline')
  const nodeSource = fleetViewModel.sources.find((source) => source.id === 'fleet_nodes')

  return (
    <div className={`fleet-focused-view fleet-focused-view--${fleetViewModel.status}`}>
      <div className="fleet-focused-view__hero">
        <div>
          <span className="fleet-focused-view__eyebrow">AULË FLEET TOPOLOGY</span>
          <h3>Fleet + Backbone</h3>
          <p>{fleetViewModel.nodes.length} projected nodes · {fleetViewModel.nodes.filter((node) => node.online).length} reachable</p>
        </div>
        <span className="fleet-focused-view__status">{fleetViewModel.status}</span>
        {onRefresh ? <button className="fleet-focused-view__refresh" onClick={onRefresh} type="button">Refresh Fleet</button> : null}
      </div>
      <div className="fleet-focused-view__topology" aria-label="Fleet topology rack line">
        {fleetViewModel.nodes.map((node, index) => (
          <button
            aria-pressed={node.id === selectedNodeId}
            className={`fleet-focused-view__topology-node fleet-focused-view__topology-node--${node.online ? 'online' : 'offline'}`}
            key={node.id}
            onClick={() => setSelectedNodeId(node.id)}
            style={{ '--fleet-node-offset': index } as CSSProperties}
            type="button"
          >
            <span>{node.online ? '●' : '○'}</span>
            <b>{node.hostname}</b>
          </button>
        ))}
      </div>
      <div className="fleet-focused-view__grid">
        <section className="fleet-focused-view__index" aria-label="Fleet node index">
          <h4>Node Index</h4>
          {fleetViewModel.nodes.map((node) => (
            <button aria-pressed={node.id === selectedNodeId} key={node.id} onClick={() => setSelectedNodeId(node.id)} type="button">
              <span>{node.displayName}</span><b>{node.online ? 'reachable' : 'offline'}</b>
            </button>
          ))}
          {fleetViewModel.nodes.length === 0 ? <p>{nodeSource?.freshness.status === 'missing' ? 'Node projection unavailable.' : 'Node projection loaded: zero nodes.'}</p> : null}
        </section>
        <section className="fleet-focused-view__detail" aria-live="polite">
          <h4>Selected Node</h4>
          {selectedNode ? <>
            <h3>{selectedNode.displayName}</h3>
            <div className="fleet-focused-view__row"><span>Reachability</span><b>{selectedNode.online ? 'reachable' : 'offline'}</b></div>
            <div className="fleet-focused-view__row"><span>Class</span><b>{selectedNode.nodeClass}</b></div>
            <div className="fleet-focused-view__row"><span>Enrollment</span><b>{selectedNode.enrollmentStatus}</b></div>
            <div className="fleet-focused-view__row"><span>Hardware</span><b>{selectedNode.hardwareSummary}</b></div>
            <div className="fleet-focused-view__row"><span>Models</span><b>{selectedNode.expectedModels.join(', ') || 'none projected'}</b></div>
            <div className="fleet-focused-view__row"><span>Backbone</span><b>{selectedNode.id === fleetViewModel.backboneNodeId ? 'primary' : 'linked'}</b></div>
            <div className="fleet-focused-view__row"><span>Routing summary</span><b>{fleetViewModel.providers.length} providers (open Routing for detail)</b></div>
          </> : <p>Select a node to inspect its evidence.</p>}
        </section>
      </div>
      <div className="fleet-focused-view__footer arda-source-corner" aria-label="Fleet source truth">
        <span>Unexpected offline: {offlineMetric?.value ?? 0}</span>
        {fleetViewModel.sources.filter((sourceRef) => sourceRef.id.startsWith('fleet_')).map((sourceRef) => <span data-truth-state={sourceRef.freshness.status} key={sourceRef.id}>{sourceRef.label}: {sourceRef.freshness.status}</span>)}
      </div>
    </div>
  )
}