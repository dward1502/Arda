import { useEffect, useState } from 'react'
import type { SystemActionId } from '../../lib/systemActionBus'
import type { RoutingViewModel } from './viewModels'

interface RoutingFocusedWorkstationViewProps {
  model: RoutingViewModel | null
  busyActionId: SystemActionId | null
  onRunAction: (actionId: SystemActionId) => void
}

export function RoutingFocusedWorkstationView({ model, busyActionId, onRunAction }: RoutingFocusedWorkstationViewProps) {
  const lanes = model?.lanes ?? []
  const [selectedLaneId, setSelectedLaneId] = useState<string | null>(lanes[0]?.lane ?? null)

  useEffect(() => {
    if (!lanes.some((lane) => lane.lane === selectedLaneId)) setSelectedLaneId(lanes[0]?.lane ?? null)
  }, [lanes, selectedLaneId])

  if (!model) {
    return <div className="routing-focused-view routing-focused-view--empty"><h3>Routing projection unavailable</h3></div>
  }

  const selectedLane = lanes.find((lane) => lane.lane === selectedLaneId) ?? null
  return (
    <div className={`routing-focused-view routing-focused-view--${model.status}`}>
      <header className="routing-focused-view__hero">
        <div>
          <span className="routing-focused-view__eyebrow">CHARON ROUTE AUTHORITY</span>
          <h3>Routing + Communications</h3>
          <p>{model.summary.join(' · ')}</p>
        </div>
        <div className="routing-focused-view__pressure">
          <span>Budget pressure</span>
          <b>{model.budgetPressure.highestLevel}</b>
          <small>{model.budgetPressure.cooldownTotal} cooldown · {model.budgetPressure.exhaustedTotal} exhausted</small>
        </div>
      </header>

      <section className="routing-focused-view__flow" aria-label="Routing lane ownership">
        {lanes.map((lane) => (
          <button
            aria-pressed={lane.lane === selectedLaneId}
            key={lane.lane}
            onClick={() => setSelectedLaneId(lane.lane)}
            type="button"
          >
            <span>{lane.label}</span>
            <i aria-hidden="true">→</i>
            <b>{lane.providerId}</b>
            <small>{lane.headroom ?? '—'} headroom</small>
          </button>
        ))}
        {lanes.length === 0 ? <p>No lane ownership projection loaded.</p> : null}
      </section>

      <div className="routing-focused-view__grid">
        <section aria-label="Selected routing lane" className="routing-focused-view__detail">
          <h4>Selected Lane</h4>
          {selectedLane ? <>
            <h3>{selectedLane.label}</h3>
            <div className="routing-focused-view__row"><span>Provider</span><b>{selectedLane.providerId}</b></div>
            <div className="routing-focused-view__row"><span>Model</span><b>{selectedLane.modelId}</b></div>
            <div className="routing-focused-view__row"><span>Route class</span><b>{selectedLane.routeClass}</b></div>
            <div className="routing-focused-view__row"><span>Reason</span><b>{selectedLane.reason || 'not projected'}</b></div>
            <div className="routing-focused-view__row"><span>Headroom / cap</span><b>{selectedLane.headroom ?? '—'} / {selectedLane.softCap ?? '—'}</b></div>
            <div className="routing-focused-view__row"><span>Fitness</span><b>{selectedLane.avgLatencyMs ?? '—'} ms · {selectedLane.successes} ok / {selectedLane.failures} fail</b></div>
          </> : <p>Select a route lane to inspect its evidence.</p>}
        </section>

        <section aria-label="Routing providers" className="routing-focused-view__providers">
          <h4>Provider Field</h4>
          {model.providers.map((provider) => (
            <div className={`routing-focused-view__provider routing-focused-view__provider--${provider.healthy ? 'healthy' : 'degraded'}`} key={provider.providerId}>
              <span>{provider.providerName}</span>
              <b>{provider.healthy ? 'healthy' : 'degraded'}</b>
              <small>{provider.activeConnections} active · {provider.modelCount} models · {provider.accessTier}</small>
            </div>
          ))}
          {model.providers.length === 0 ? <p>Provider projection unavailable.</p> : null}
        </section>

        <section aria-label="Communication pathways" className="routing-focused-view__communications">
          <h4>Communication Paths</h4>
          {model.communicationPathways.map((pathway) => (
            <div className="routing-focused-view__row" key={pathway.id}><span>{pathway.label}</span><b>{pathway.state} · {pathway.receipts} receipt{pathway.receipts === 1 ? '' : 's'}</b></div>
          ))}
          {model.communicationPathways.length === 0 ? <p>No Discord, Telegram, or gateway pathway receipts loaded.</p> : null}
        </section>

        <section aria-label="Routing actions" className="routing-focused-view__actions">
          <h4>Read-only Actions</h4>
          {model.actions.map((action) => {
            const actionId = action.id as SystemActionId
            return <button disabled={busyActionId === actionId} key={action.id} onClick={() => onRunAction(actionId)} type="button">{busyActionId === actionId ? 'Running…' : action.label}</button>
          })}
          <p>Provider reroute and route mutation are not exposed.</p>
        </section>
      </div>

      <footer className="routing-focused-view__footer arda-source-corner" aria-label="Routing source truth">
        <span>Routes: {model.routeHistory.successes} success / {model.routeHistory.failures} failure</span>
        {model.sources.map((source) => <span data-truth-state={source.freshness.status} key={source.id}>{source.label}: {source.freshness.status}</span>)}
      </footer>
    </div>
  )
}
