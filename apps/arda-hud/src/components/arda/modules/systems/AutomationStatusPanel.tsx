// sigil: REPAIR
import { primarySigilForSource } from '../../../../lib/soterionRender'

interface AutomationStatusCheck {
  name: string
  status: string
  detail: string
}

interface AutomationStatusSurface {
  posture?: string
  execution_allowed?: boolean
  eval?: {
    present?: boolean
    status?: string
    score?: number | null
    evaluated_receipts?: number
    failed_receipts?: number
    generated_at_utc?: string
  }
  hold?: {
    present?: boolean
    reason?: string
    mutate_task_queue?: boolean
    blocked_actions?: string[]
  }
  top_checks?: AutomationStatusCheck[]
  evidence_sources?: string[]
}

interface AutomationStatusPanelProps {
  status: AutomationStatusSurface | null
}

function formatScore(score: number | null | undefined): string {
  return typeof score === 'number' && Number.isFinite(score) ? score.toFixed(2) : 'n/a'
}

function chipClass(status: string | undefined): string {
  if (status === 'pass' || status === 'passing') return 'systems-chip--good'
  if (status === 'fail' || status === 'held') return 'systems-chip--warn'
  return 'systems-chip--muted'
}

export default function AutomationStatusPanel({ status }: AutomationStatusPanelProps) {
  const prometheusMarker = primarySigilForSource('prometheus')
  const posture = status?.posture ?? 'unknown'
  const evalStatus = status?.eval?.status ?? 'missing'
  const checks = status?.top_checks ?? []
  const evidenceSources = status?.evidence_sources ?? []

  return (
    <section className="systems-panel systems-panel--health">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">{prometheusMarker} Automation Status</div>
        <h3 className="systems-panel__title">Daily Eval + Hold</h3>
      </header>
      <div className="systems-kpi-grid">
        <article className={`systems-kpi ${status?.execution_allowed ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">Execution</span>
          <strong className="systems-kpi__value">{status?.execution_allowed ? 'allowed' : 'blocked'}</strong>
        </article>
        <article className={`systems-kpi ${posture === 'passing' ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">Posture</span>
          <strong className="systems-kpi__value">{posture}</strong>
        </article>
      </div>
      <div className="systems-chip-cloud">
        <span className={`systems-chip ${chipClass(evalStatus)}`}>eval {evalStatus}</span>
        <span className={`systems-chip ${status?.hold?.present ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          hold {status?.hold?.present ? 'present' : 'clear'}
        </span>
        <span className="systems-chip systems-chip--muted">score {formatScore(status?.eval?.score)}</span>
        <span className="systems-chip systems-chip--muted">failed {status?.eval?.failed_receipts ?? 0}</span>
      </div>
      {status?.hold?.present && (
        <p className="systems-panel__note">
          Hold reason: {status.hold.reason || 'reason unavailable'} · queue mutation {status.hold.mutate_task_queue ? 'allowed' : 'blocked'}
        </p>
      )}
      <div className="document-list compact">
        {checks.length > 0 ? checks.map((check) => (
          <article className="document-list__item" key={`${check.name}-${check.status}`}>
            <div className="document-list__title-row">
              <strong>{check.name}</strong>
              <span className={`systems-chip ${chipClass(check.status)}`}>{check.status}</span>
            </div>
            {check.detail && <p>{check.detail}</p>}
          </article>
        )) : <span className="systems-empty">No daily eval checks available.</span>}
      </div>
      {evidenceSources.length > 0 && (
        <p className="systems-panel__note">Evidence: {evidenceSources.join(' · ')}</p>
      )}
    </section>
  )
}
