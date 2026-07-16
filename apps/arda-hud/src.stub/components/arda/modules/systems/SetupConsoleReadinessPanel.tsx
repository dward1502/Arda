// sigil: REPAIR
import type { JsonRecord } from '../../../../lib/ardaSource'
import { primarySigilForSource } from '../../../../lib/soterionRender'

interface SetupConsoleReadinessPanelProps {
  readiness: JsonRecord | null
  guidedSession?: JsonRecord | null
  privateConfigStage?: JsonRecord | null
  servicePlan?: JsonRecord | null
}

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as JsonRecord
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function stringValue(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback
}

function numberValue(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function nullableNumberValue(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function boolValue(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback
}

function statusClass(status: string): string {
  if (status === 'pass') return 'systems-chip--good'
  if (status === 'warn') return 'systems-chip--warn'
  if (status === 'fail') return 'systems-chip--warn'
  return 'systems-chip--accent'
}

export default function SetupConsoleReadinessPanel({
  readiness,
  guidedSession,
  privateConfigStage,
  servicePlan,
}: SetupConsoleReadinessPanelProps) {
  const checks = asArray(readiness?.checks)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
  const summary = asRecord(readiness?.summary)
  const passCount = numberValue(summary?.pass)
  const warnCount = numberValue(summary?.warn)
  const failCount = numberValue(summary?.fail)
  const gateStatus = stringValue(readiness?.gate_status, readiness ? 'unknown' : 'missing')
  const mode = stringValue(readiness?.mode, 'not generated')
  const portabilityStatus = asRecord(readiness?.portability_status)
  const portabilityBlockers = nullableNumberValue(portabilityStatus?.active_blocker_findings)
  const portabilityLabel = stringValue(portabilityStatus?.label, 'portability projection unavailable')
  const portabilityState = stringValue(portabilityStatus?.status, portabilityBlockers === 0 ? 'pass' : 'unknown')
  const guidedSteps = asArray(guidedSession?.steps)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
  const nextActions = asArray(guidedSession?.next_actions).map((item) => stringValue(item)).filter(Boolean)
  const privateEntries = asArray(privateConfigStage?.entries)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
  const missingRequired = asArray(privateConfigStage?.missing_required)
  const serviceActions = asArray(servicePlan?.actions)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
  const humanGatedActions = serviceActions.filter((action) => boolValue(action.requires_human_gate))
  const privateBaselineAction = humanGatedActions.find((action) => stringValue(action.action_id) === 'onboarding.write_private_config_baseline')
  const setupMarker = primarySigilForSource('hades')
  const visibleChecks = checks
    .filter((check) => stringValue(check.status) !== 'pass')
    .concat(checks.filter((check) => stringValue(check.status) === 'pass'))
    .slice(0, 4)

  return (
    <section className="systems-panel systems-panel--health">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">{setupMarker} Setup Console</div>
        <h3 className="systems-panel__title">Readiness Audit</h3>
      </header>
      <div className="systems-kpi-grid">
        <article className={`systems-kpi ${gateStatus === 'pass' ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">Gate</span>
          <strong className="systems-kpi__value">{gateStatus}</strong>
        </article>
        <article className="systems-kpi systems-kpi--accent">
          <span className="systems-kpi__label">Mode</span>
          <strong className="systems-kpi__value">{mode}</strong>
        </article>
        <article className={`systems-kpi ${portabilityState === 'pass' ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">Portability blockers</span>
          <strong className="systems-kpi__value">{portabilityBlockers ?? '—'}</strong>
          <span className="systems-kpi__note">{portabilityLabel}</span>
        </article>
        <article className={`systems-kpi ${nextActions.length === 0 ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">First Light next</span>
          <strong className="systems-kpi__value">{nextActions.length}</strong>
          <span className="systems-kpi__note">{guidedSteps.length} guided steps</span>
        </article>
      </div>
      <div className="systems-chip-cloud">
        <span className="systems-chip systems-chip--good">pass {passCount}</span>
        <span className="systems-chip systems-chip--warn">warn {warnCount}</span>
        <span className={`systems-chip ${failCount > 0 ? 'systems-chip--warn' : 'systems-chip--accent'}`}>fail {failCount}</span>
        <span className={`systems-chip ${missingRequired.length > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          config missing {missingRequired.length}
        </span>
        <span className="systems-chip systems-chip--warn">human gates {humanGatedActions.length}</span>
      </div>
      {(guidedSession || privateConfigStage || servicePlan) && (
        <div className="document-list compact">
          <article className="document-list__item">
            <div className="document-list__title-row">
              <strong>First Light artifacts</strong>
              <span className="systems-chip systems-chip--accent">local</span>
            </div>
            <p>
              Guided session: {guidedSteps.length || 'missing'} steps; private config entries: {privateEntries.length || 'missing'}; service actions:{' '}
              {serviceActions.length || 'missing'}.
            </p>
            <p>
              {privateBaselineAction
                ? `${stringValue(privateBaselineAction.title)} requires scoped human approval before private env writes.`
                : 'Private config baseline action is not present in the latest service plan.'}
            </p>
          </article>
          {nextActions.slice(0, 3).map((nextAction) => (
            <article className="document-list__item" key={nextAction}>
              <div className="document-list__title-row">
                <strong>Next action</strong>
                <span className="systems-chip systems-chip--warn">review</span>
              </div>
              <p>{nextAction}</p>
            </article>
          ))}
        </div>
      )}
      <div className="document-list compact">
        {visibleChecks.length > 0 ? (
          visibleChecks.map((check) => {
            const checkId = stringValue(check.check_id, 'unknown')
            const status = stringValue(check.status, 'unknown')
            return (
              <article className="document-list__item" key={checkId}>
                <div className="document-list__title-row">
                  <strong>{checkId}</strong>
                  <span className={`systems-chip ${statusClass(status)}`}>{status}</span>
                </div>
                <p>{stringValue(check.title, 'untitled check')}</p>
                <p>{stringValue(check.recommendation, 'No recommendation recorded.')}</p>
              </article>
            )
          })
        ) : (
          <span className="systems-empty">Run arda-cli onboarding readiness to publish readiness state.</span>
        )}
      </div>
    </section>
  )
}
