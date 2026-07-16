// sigil: REPAIR
import type { ReactNode } from 'react'

interface SystemStatusMessageProps {
  bundleError?: string | null
  charonLiveError?: string | null
  bundleLoading?: boolean
}

function SystemStatusMessage({
  bundleError,
  charonLiveError,
  bundleLoading,
}: SystemStatusMessageProps): ReactNode {
  if (bundleLoading) {
    return <span className="system-status-message">Loading core-state bundle...</span>
  }

  if (charonLiveError) {
    return <span className="system-status-message system-status-message--warning">Charon live: {charonLiveError}</span>
  }

  if (bundleError) {
    return <span className="system-status-message system-status-message--warning">Core bundle: {bundleError}</span>
  }

  return null
}

interface OperatingSurfaceRailItem {
  lane: string
  subtitle: string
  panelModeKey: string
  report?: {
    status?: string
    current?: string
  }
}

interface OperatingSurfaceRailProps {
  items: OperatingSurfaceRailItem[]
  activePanelModeKey: string
  openLane: (panelModeKey: string, lane: OperatingSurfaceRailItem['lane']) => void
  nowReport?: {
    status?: string
    current?: string
  }
  workReport?: {
    status?: string
    current?: string
  }
  decisionsReport?: {
    status?: string
    current?: string
  }
  healthReport?: {
    status?: string
    current?: string
  }
  liveRuntimeRailStatus: string
  liveRuntime?: {
    source?: string
    sequence?: number
    lastEventIso?: string
    status?: string
  }
  bundleError?: string | null
  charonLiveError?: string | null
  bundleLoading?: boolean
}

export default function OperatingSurfaceRail({
  items,
  activePanelModeKey,
  openLane,
  nowReport,
  workReport,
  decisionsReport,
  healthReport,
  liveRuntimeRailStatus,
  liveRuntime,
  bundleError,
  charonLiveError,
  bundleLoading,
}: OperatingSurfaceRailProps) {
  return (
    <nav className="operating-surface-rail" aria-label="ARDA operating surface navigation">
      <div className="operating-surface-rail__brief">
        <span className="operating-surface-rail__eyebrow">Operating Surface</span>
        <strong>Now: {nowReport?.status ?? 'loading'}</strong>
        <span title={nowReport?.current ?? 'Waiting for core-state bundle.'}>
          {nowReport?.current ?? 'Waiting for core-state bundle.'}
        </span>
        <SystemStatusMessage
          bundleError={bundleError}
          charonLiveError={charonLiveError}
          bundleLoading={bundleLoading}
        />
      </div>
      <div className="operating-surface-rail__attention" aria-label="Current operator attention">
        <span>Work: {workReport?.status ?? 'loading'}</span>
        <span>Decisions: {decisionsReport?.status ?? 'loading'}</span>
        <span>Health: {healthReport?.status ?? 'loading'}</span>
        <span
          className={`operating-surface-rail__pulse operating-surface-rail__pulse--${liveRuntimeRailStatus}`}
          title={
            liveRuntime
              ? `${liveRuntime.source} event ${liveRuntime.sequence} at ${liveRuntime.lastEventIso}`
              : 'Waiting for live runtime channel pulse'
          }
        >
          Pulse: {liveRuntime ? `${liveRuntime.status} #${liveRuntime.sequence}` : 'idle'}
        </span>
      </div>
      <div className="operating-surface-rail__lanes">
        {items.map((item) => {
          const status = item.report?.status ?? 'partial'
          const isActive = activePanelModeKey === item.panelModeKey

          return (
            <button
              type="button"
              key={item.lane}
              className={`operating-surface-rail__lane operating-surface-rail__lane--${status}`}
              onClick={() => openLane(item.panelModeKey, item.lane)}
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
    </nav>
  )
}
