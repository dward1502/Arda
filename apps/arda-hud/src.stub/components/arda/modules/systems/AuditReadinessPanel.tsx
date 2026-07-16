// sigil: REPAIR
import { primarySigilForSource } from '../../../../lib/soterionRender'

interface AuditReadinessPanelProps {
  readiness: Record<string, unknown> | null
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function getString(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback
}

function getNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function chipClass(status: string): string {
  if (status === 'closed' || status === 'completed') return 'systems-chip--good'
  if (status === 'in_progress' || status === 'queued') return 'systems-chip--warn'
  return 'systems-chip--muted'
}

export default function AuditReadinessPanel({ readiness }: AuditReadinessPanelProps) {
  const auditMarker = primarySigilForSource('audit')
  const phase7 = asRecord(readiness?.phase7_closeout)
  const phase8 = asRecord(readiness?.phase8_hardening)
  const boundary = asRecord(readiness?.boundary)
  const nextItems = asArray(readiness?.next_items)
    .map((item) => asRecord(item))
    .filter((item): item is Record<string, unknown> => item !== null)
  const evidenceSources = asArray(readiness?.evidence_sources).map((item) => getString(item)).filter(Boolean)

  const phase7Status = getString(phase7?.status, 'unknown')
  const hardeningQueued = getNumber(phase8?.queued, 0)
  const hardeningInProgress = getNumber(phase8?.in_progress, 0)
  const hardeningCompleted = getNumber(phase8?.completed, 0)

  return (
    <section className="systems-panel systems-panel--health">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">{auditMarker} Audit Readiness</div>
        <h3 className="systems-panel__title">Professionalization Closeout</h3>
      </header>
      <div className="systems-kpi-grid">
        <article className={`systems-kpi ${phase7Status === 'closed' ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">Phase 7 remediation</span>
          <strong className="systems-kpi__value">{phase7Status}</strong>
        </article>
        <article className={`systems-kpi ${hardeningQueued > 0 || hardeningInProgress > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Phase 8 hardening queue</span>
          <strong className="systems-kpi__value">{hardeningQueued + hardeningInProgress}</strong>
        </article>
      </div>
      <div className="systems-chip-cloud">
        <span className={`systems-chip ${chipClass(phase7Status)}`}>phase 7 {phase7Status}</span>
        <span className="systems-chip systems-chip--muted">slices {getNumber(phase7?.verified_slices, 0)}/{getNumber(phase7?.total_slices, 0)}</span>
        <span className="systems-chip systems-chip--good">completed {hardeningCompleted}</span>
        <span className="systems-chip systems-chip--warn">queued {hardeningQueued}</span>
      </div>
      <p className="systems-panel__note">
        {getString(boundary?.summary, 'Audit evidence only; this is not live runtime/service status.')}
      </p>
      <p className="systems-panel__note">
        {getString(boundary?.roadmap, 'Future embodied roadmap work remains separate from closed audit remediation.')}
      </p>
      <div className="document-list compact">
        {nextItems.length > 0 ? nextItems.map((item) => (
          <article className="document-list__item" key={getString(item.id, getString(item.title, 'audit-next-item'))}>
            <div className="document-list__title-row">
              <strong>{getString(item.id, 'next')} · {getString(item.title, 'Untitled hardening item')}</strong>
              <span className={`systems-chip ${chipClass(getString(item.status, 'unknown'))}`}>{getString(item.status, 'unknown')}</span>
            </div>
            <p>{getString(item.scope, 'scope unavailable')}</p>
          </article>
        )) : <span className="systems-empty">No Phase 8 hardening items remain queued.</span>}
      </div>
      {evidenceSources.length > 0 && (
        <p className="systems-panel__note">Evidence: {evidenceSources.join(' · ')}</p>
      )}
    </section>
  )
}
