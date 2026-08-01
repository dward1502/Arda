import { useEffect, useState } from 'react'
import type { RunGraph, WorkbenchEvent } from '../../lib/workbench'

interface RunTimelineProps { events: WorkbenchEvent[]; graph: RunGraph | null }

function eventLabel(event: WorkbenchEvent): string {
  const detail = event.kind ?? event.event ?? event
  return detail.type?.replace(/_/g, ' ') ?? 'run event'
}

export default function RunTimeline({ events, graph }: RunTimelineProps) {
  const [reducedMotion, setReducedMotion] = useState(false)
  useEffect(() => {
    const query = window.matchMedia?.('(prefers-reduced-motion: reduce)')
    if (!query) return
    const update = () => setReducedMotion(query.matches)
    update()
    query.addEventListener?.('change', update)
    return () => query.removeEventListener?.('change', update)
  }, [])
  const maxCost = graph?.nodes.reduce((total, node) => total + node.budget.max_cost_usd, 0) ?? 0
  const recoveryToken = graph?.nodes.map((node) => node.checkpoint.recovery_token).find(Boolean) ?? null
  return (
    <section className={`workbench-panel workbench-timeline${reducedMotion ? ' workbench-timeline--reduced-motion' : ''}`} aria-labelledby="run-timeline-title">
      <header><h3 id="run-timeline-title">Timeline and receipt</h3><span>Budget ceiling ${maxCost.toFixed(2)}</span></header>
      <dl className="workbench-receipt"><div><dt>Cost</dt><dd>${maxCost.toFixed(2)} maximum</dd></div><div><dt>Resume</dt><dd>{recoveryToken ?? (graph ? `Reload run ${graph.run_id}` : 'No run to resume')}</dd></div></dl>
      {events.length === 0 ? <p>No run events recorded.</p> : <ol className="workbench-events">{events.map((event, index) => {
        const recordedAt = event.recorded_at_unix_ms ?? event.occurred_at_unix_ms
        return <li key={`${event.sequence ?? index}-${eventLabel(event)}`}><strong>{eventLabel(event)}</strong><span>{event.node_id ?? 'run'}</span>{recordedAt ? <time dateTime={new Date(recordedAt).toISOString()}>{new Date(recordedAt).toLocaleString()}</time> : null}</li>
      })}</ol>}
    </section>
  )
}
