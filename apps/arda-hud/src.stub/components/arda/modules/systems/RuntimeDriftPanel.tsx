// sigil: REPAIR
import { primarySigilForSource } from '../../../../lib/soterionRender'

interface RuntimeDriftItem {
  nodeId: string
  displayName: string
  providerId: string
  declaredModel: string
  declaredContextWindow: number | null
  charonContextWindow: number | null
  actualProcessContextWindow: number | null
  declaredVsCharon: boolean
  declaredVsLocalProcess: boolean
  localRuntimeStatus: string
}

interface RuntimeDriftPanelProps {
  totalNodes: number
  driftedNodes: number
  items: RuntimeDriftItem[]
}

function formatWindow(value: number | null): string {
  return value && value > 0 ? `${value.toLocaleString()} ctx` : 'n/a'
}

function formatProvider(providerId: string): string {
  return providerId ? providerId.split('_').join(' ') : 'unmapped'
}

export default function RuntimeDriftPanel({ totalNodes, driftedNodes, items }: RuntimeDriftPanelProps) {
  const driftItems = items.filter((item) => item.declaredVsCharon || item.declaredVsLocalProcess)
  const visibleItems = (driftItems.length > 0 ? driftItems : items).slice(0, 4)
  const charonMarker = primarySigilForSource('charon')

  return (
    <section className="systems-panel systems-panel--health">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">{charonMarker} Runtime Drift</div>
        <h3 className="systems-panel__title">Declared Vs Live</h3>
      </header>
      <div className="systems-kpi-grid">
        <article className={`systems-kpi ${driftedNodes > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Drifted Nodes</span>
          <strong className="systems-kpi__value">{driftedNodes}/{totalNodes}</strong>
        </article>
        <article className="systems-kpi systems-kpi--accent">
          <span className="systems-kpi__label">Declared Source</span>
          <strong className="systems-kpi__value">fleet.toml</strong>
        </article>
      </div>
      <div className="document-list compact">
        {visibleItems.length > 0 ? (
          visibleItems.map((item) => (
            <article className="document-list__item" key={item.nodeId}>
              <div className="document-list__title-row">
                <strong>{item.displayName}</strong>
                <span className={`systems-chip ${item.declaredVsCharon || item.declaredVsLocalProcess ? 'systems-chip--warn' : 'systems-chip--good'}`}>
                  {item.declaredVsCharon || item.declaredVsLocalProcess ? 'drift' : 'aligned'}
                </span>
              </div>
              <p>{formatProvider(item.providerId)} · {item.declaredModel || 'model unset'}</p>
              <p>
                declared {formatWindow(item.declaredContextWindow)} / charon {formatWindow(item.charonContextWindow)} / process {formatWindow(item.actualProcessContextWindow)}
              </p>
              {(item.declaredVsCharon || item.declaredVsLocalProcess) && (
                <div className="systems-chip-cloud">
                  {item.declaredVsCharon && <span className="systems-chip systems-chip--warn">charon mismatch</span>}
                  {item.declaredVsLocalProcess && <span className="systems-chip systems-chip--warn">process mismatch</span>}
                </div>
              )}
            </article>
          ))
        ) : (
          <span className="systems-empty">No runtime drift data.</span>
        )}
      </div>
    </section>
  )
}
