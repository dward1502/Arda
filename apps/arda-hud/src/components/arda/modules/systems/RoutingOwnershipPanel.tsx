// sigil: REPAIR
interface LaneRoute {
  providerId: string
  modelId: string
  routeClass: string
  reason: string
}

interface RoutingOwnershipPanelProps {
  lanes: Array<{
    lane: string
    priority: string
    route: LaneRoute | null
  }>
}

export default function RoutingOwnershipPanel({ lanes }: RoutingOwnershipPanelProps) {
  return (
    <section className="systems-panel systems-panel--routing">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">Charon Lanes</div>
        <h3 className="systems-panel__title">Routing Ownership</h3>
      </header>
      <div className="lane-route-list">
        {lanes.map((entry) => (
          <article className="lane-route-card" key={entry.lane}>
            <div className="lane-route-card__topline">
              <span className="lane-route-card__lane">{entry.lane}</span>
              <span className="lane-route-card__priority">{entry.priority}</span>
            </div>
            <strong className="lane-route-card__provider">{entry.route?.providerId ?? 'unassigned'}</strong>
            <span className="lane-route-card__model">{entry.route?.modelId ?? 'no model'}</span>
            <p className="lane-route-card__reason">{entry.route?.routeClass ?? 'route unavailable'}</p>
          </article>
        ))}
      </div>
    </section>
  )
}
