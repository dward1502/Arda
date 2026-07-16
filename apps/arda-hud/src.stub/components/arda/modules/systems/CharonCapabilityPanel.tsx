// sigil: REPAIR
import type { CharonLiveSnapshot, CharonPromotionCandidateView } from '../../../../lib/charonLive'

interface CharonCapabilityPanelProps {
  snapshot: CharonLiveSnapshot | null
  error: string | null
  loading: boolean
  storagePressure: Record<string, unknown> | null
}

function chipClass(value: string) {
  if (value === 'passed' || value === 'observed' || value === 'ready' || value === 'ok') return 'systems-chip--good'
  if (value === 'failed' || value === 'blocked' || value === 'warn' || value === 'critical') return 'systems-chip--warn'
  return 'systems-chip--muted'
}

function candidateStatus(candidate: CharonPromotionCandidateView): string {
  if (candidate.promotion_ready) return 'ready'
  if (candidate.status === 'rejected') return 'rejected'
  if (candidate.reasons.includes('adapter_required')) return 'adapter'
  if (candidate.reasons.includes('needs_passive_receipt_or_operator_approved_active_probe')) return 'evidence'
  return candidate.status || 'candidate'
}

function formatTime(value: string | null | undefined): string {
  if (!value) return 'not observed'
  const parsed = Date.parse(value)
  if (Number.isNaN(parsed)) return value
  return new Date(parsed).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function numberValue(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function stringValue(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback
}

export default function CharonCapabilityPanel({ snapshot, error, loading, storagePressure }: CharonCapabilityPanelProps) {
  const summary = snapshot?.capabilities?.capabilities.summary
  const health = snapshot?.health
  const providers = snapshot?.capabilities?.capabilities.providers ?? []
  const candidates = snapshot?.providerCandidates?.promotion_guard.candidates ?? []
  const pressureProviders = health?.budget_pressure.providers
    ?.filter((provider) => provider.level !== 'ok' || provider.in_cooldown)
    .slice(0, 6) ?? []
  const storageSummary = asRecord(storagePressure?.summary)
  const storageClasses = asArray(storagePressure?.classes)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is Record<string, unknown> => entry !== null)
  const storageCandidateCount = numberValue(storageSummary?.cleanup_candidate_count)
  const misplacedModelCount = numberValue(storageSummary?.misplaced_model_artifact_count)
  const storageStatus = stringValue(storagePressure?.status, 'unknown')
  const storageWarnings = storageClasses
    .filter((entry) => ['generated_tick_output', 'runtime_backup', 'rebuildable_temp', 'runtime_log', 'misplaced_model_artifact'].includes(stringValue(entry.category)))
    .slice(0, 5)
  const observedProviders = providers
    .filter((provider) => provider.enabled)
    .sort((left, right) => {
      if (left.evidence_state === right.evidence_state) return left.provider_id.localeCompare(right.provider_id)
      return left.evidence_state === 'observed' ? -1 : 1
    })
    .slice(0, 6)
  const visibleCandidates = candidates.slice(0, 6)
  const liveState = error ? 'blocked' : loading && !snapshot ? 'loading' : snapshot ? 'observed' : 'unknown'

  return (
    <section className="systems-panel systems-panel--routing">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">Charon Capability Stream</div>
        <h3 className="systems-panel__title">Model Receipts + Promotion Guard</h3>
      </header>
      <div className="systems-kpi-grid">
        <article className={`systems-kpi ${error ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Live Feed</span>
          <strong className="systems-kpi__value">{liveState}</strong>
          <span className="systems-kpi__note">{error || `updated ${formatTime(snapshot?.loadedAt)}`}</span>
        </article>
        <article className="systems-kpi systems-kpi--accent">
          <span className="systems-kpi__label">Receipt Models</span>
          <strong className="systems-kpi__value">{summary?.receipt_model_count ?? 0}</strong>
          <span className="systems-kpi__note">passive evidence</span>
        </article>
        <article className={`systems-kpi ${(summary?.recent_capability_failures ?? 0) > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Recent Failures</span>
          <strong className="systems-kpi__value">{summary?.recent_capability_failures ?? 0}</strong>
          <span className="systems-kpi__note">tool/json/streaming</span>
        </article>
        <article className="systems-kpi systems-kpi--idle">
          <span className="systems-kpi__label">Unknown Providers</span>
          <strong className="systems-kpi__value">{summary?.providers_with_no_capability_evidence ?? 0}</strong>
          <span className="systems-kpi__note">no passive receipt yet</span>
        </article>
        <article className={`systems-kpi ${(health?.budget_pressure.highest_level ?? 'ok') === 'ok' ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">Budget Pressure</span>
          <strong className="systems-kpi__value">{health?.budget_pressure.highest_level ?? 'unknown'}</strong>
          <span className="systems-kpi__note">{health?.providers_healthy ?? 0} healthy / {health?.providers_ready ?? 0} ready</span>
        </article>
        <article className={`systems-kpi ${storageStatus === 'ok' ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">HADES Storage</span>
          <strong className="systems-kpi__value">{storageStatus}</strong>
          <span className="systems-kpi__note">{storageCandidateCount} candidates / {misplacedModelCount} misplaced models</span>
        </article>
      </div>
      <div className="systems-chip-cloud">
        <span className="systems-chip systems-chip--muted">tool failed {summary?.models_with_failed_tool_receipts ?? 0}</span>
        <span className="systems-chip systems-chip--muted">json failed {summary?.models_with_failed_structured_output_receipts ?? 0}</span>
        <span className="systems-chip systems-chip--muted">stream failed {summary?.models_with_failed_streaming_receipts ?? 0}</span>
        <span className="systems-chip systems-chip--accent">tool floor {health?.route_guardrails.tool_execution_min_context_window ?? 0}</span>
        <span className="systems-chip systems-chip--muted">visible reasoning {health?.route_guardrails.visible_reasoning_model_total ?? 0}</span>
        <span className={`systems-chip ${snapshot?.providerCandidates?.promotion_guard.active_capability_probes_enabled ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          active probes {snapshot?.providerCandidates?.promotion_guard.active_capability_probes_enabled ? 'on' : 'off'}
        </span>
      </div>
      <div className="systems-detail-grid">
        <div className="systems-detail-block">
          <div className="systems-detail-block__label">Provider Evidence</div>
          <div className="systems-chip-cloud">
            {observedProviders.length > 0 ? observedProviders.map((provider) => (
              <span className={`systems-chip ${chipClass(provider.evidence_state)}`} key={provider.provider_id}>
                {provider.provider_id} {provider.evidence_state}
              </span>
            )) : <span className="systems-empty">No live provider evidence yet.</span>}
          </div>
        </div>
        <div className="systems-detail-block">
          <div className="systems-detail-block__label">Provider Pressure</div>
          <div className="systems-chip-cloud">
            {pressureProviders.length > 0 ? pressureProviders.map((provider) => (
              <span className={`systems-chip ${chipClass(provider.level)}`} key={provider.provider_id}>
                {provider.provider_id} {provider.level}{provider.in_cooldown ? ' cooldown' : ''}
              </span>
            )) : <span className="systems-empty">No provider pressure warnings.</span>}
          </div>
        </div>
        <div className="systems-detail-block">
          <div className="systems-detail-block__label">Storage Warnings</div>
          <div className="systems-chip-cloud">
            {storageWarnings.length > 0 ? storageWarnings.map((entry) => (
              <span className={`systems-chip ${chipClass(storageStatus)}`} key={stringValue(entry.category)}>
                {stringValue(entry.category)} {numberValue(entry.files)} files
              </span>
            )) : <span className="systems-empty">No HADES storage warnings.</span>}
          </div>
        </div>
        <div className="systems-detail-block">
          <div className="systems-detail-block__label">Candidate Guard</div>
          <div className="systems-chip-cloud">
            {visibleCandidates.length > 0 ? visibleCandidates.map((candidate) => (
              <span className={`systems-chip ${candidate.promotion_ready ? 'systems-chip--good' : 'systems-chip--warn'}`} key={candidate.id}>
                {candidate.id} {candidateStatus(candidate)}
              </span>
            )) : <span className="systems-empty">No provider candidates loaded.</span>}
          </div>
        </div>
      </div>
    </section>
  )
}
