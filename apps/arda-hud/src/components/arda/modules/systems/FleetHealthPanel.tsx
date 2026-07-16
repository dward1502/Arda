// sigil: REPAIR
interface FleetHealthPanelProps {
  totalTargets: number
  liveTargets: number
  routableProviders: number
  intentionalOffline: number
  unexpectedOffline: number
  intentionalOfflineTargets: Array<{ displayName: string; providerId: string }>
  unexpectedOfflineTargets: Array<{ displayName: string; providerId: string }>
}

function formatProvider(providerId: string): string {
  return providerId.split('_').join(' ')
}

export default function FleetHealthPanel({
  totalTargets,
  liveTargets,
  routableProviders,
  intentionalOffline,
  unexpectedOffline,
  intentionalOfflineTargets,
  unexpectedOfflineTargets,
}: FleetHealthPanelProps) {
  return (
    <section className="systems-panel systems-panel--health">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">Fleet Health</div>
        <h3 className="systems-panel__title">Mesh Status</h3>
      </header>
      <div className="systems-kpi-grid">
        <article className="systems-kpi systems-kpi--good">
          <span className="systems-kpi__label">Live Nodes</span>
          <strong className="systems-kpi__value">{liveTargets}/{totalTargets}</strong>
        </article>
        <article className="systems-kpi systems-kpi--accent">
          <span className="systems-kpi__label">Routable</span>
          <strong className="systems-kpi__value">{routableProviders}</strong>
        </article>
        <article className="systems-kpi systems-kpi--idle">
          <span className="systems-kpi__label">Intentional Offline</span>
          <strong className="systems-kpi__value">{intentionalOffline}</strong>
        </article>
        <article className={`systems-kpi ${unexpectedOffline > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Unexpected Offline</span>
          <strong className="systems-kpi__value">{unexpectedOffline}</strong>
        </article>
      </div>
      <div className="systems-detail-grid">
        <div className="systems-detail-block">
          <div className="systems-detail-block__label">Planned Absences</div>
          <div className="systems-chip-cloud">
            {intentionalOfflineTargets.length > 0 ? (
              intentionalOfflineTargets.map((target) => (
                <span className="systems-chip systems-chip--idle" key={target.providerId}>
                  {target.displayName}
                </span>
              ))
            ) : (
              <span className="systems-empty">None</span>
            )}
          </div>
        </div>
        <div className="systems-detail-block">
          <div className="systems-detail-block__label">Unexpected Losses</div>
          <div className="systems-chip-cloud">
            {unexpectedOfflineTargets.length > 0 ? (
              unexpectedOfflineTargets.map((target) => (
                <span className="systems-chip systems-chip--warn" key={target.providerId}>
                  {target.displayName} / {formatProvider(target.providerId)}
                </span>
              ))
            ) : (
              <span className="systems-empty">None</span>
            )}
          </div>
        </div>
      </div>
    </section>
  )
}
