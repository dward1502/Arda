// sigil: REPAIR
interface LaneHeadroomPanelProps {
  providers: Array<{
    providerId: string
    softCaps: Record<string, number>
    laneHeadroom: Record<string, number>
  }>
}

const LANES = ['interactive', 'execution', 'background'] as const

export default function LaneHeadroomPanel({ providers }: LaneHeadroomPanelProps) {
  return (
    <section className="systems-panel systems-panel--headroom">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">Queue Pressure</div>
        <h3 className="systems-panel__title">Lane Headroom</h3>
      </header>
      <div className="headroom-provider-list">
        {providers.map((provider) => (
          <article className="headroom-provider-card" key={provider.providerId}>
            <strong className="headroom-provider-card__title">{provider.providerId}</strong>
            <div className="headroom-provider-card__rows">
              {LANES.map((lane) => {
                const headroom = provider.laneHeadroom[lane] ?? 0
                const width = Math.max(8, Math.round(headroom * 100))
                return (
                  <div className="headroom-row" key={`${provider.providerId}-${lane}`}>
                    <span className="headroom-row__label">{lane}</span>
                    <div className="headroom-row__bar">
                      <div className="headroom-row__fill" style={{ width: `${width}%` }} />
                    </div>
                    <span className="headroom-row__value">{Math.round(headroom * 100)}%</span>
                    <span className="headroom-row__cap">cap {provider.softCaps[lane] ?? 0}</span>
                  </div>
                )
              })}
            </div>
          </article>
        ))}
      </div>
    </section>
  )
}
