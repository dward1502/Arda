import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { getRefreshAffordance } from '../../../lib/ardaProvenance'

interface SourceRefreshAffordanceProps {
  record: ArdaSourceProvenance
  compact?: boolean
}

function safetyLabel(safety: ReturnType<typeof getRefreshAffordance>['safety']): string {
  switch (safety) {
    case 'read_only':
      return 'Read-only'
    case 'projection_refresh_only':
      return 'Projection refresh'
    case 'approval_required':
      return 'Approval required'
    case 'not_registered':
      return 'Manual only'
    default:
      return 'Manual only'
  }
}

export function SourceRefreshAffordance({ record, compact = false }: SourceRefreshAffordanceProps) {
  const affordance = getRefreshAffordance(record)

  if (compact && affordance.safety === 'not_registered') {
    return null
  }

  if (compact) {
    return (
      <span className={`source-refresh-affordance source-refresh-affordance--${affordance.safety}`}>
        {safetyLabel(affordance.safety)}
      </span>
    )
  }

  return (
    <section className={`source-refresh-affordance-card source-refresh-affordance-card--${affordance.safety}`}>
      <div className="source-refresh-affordance-card__header">
        <div>
          <strong>{affordance.label}</strong>
          <span>{safetyLabel(affordance.safety)} / display-only</span>
        </div>
        {affordance.systemActionId ? <code>{affordance.systemActionId}</code> : null}
      </div>
      {affordance.command ? <code className="source-refresh-affordance-card__command">{affordance.command}</code> : null}
      <p>{affordance.operatorInstruction}</p>
      <small>{affordance.notes}</small>
    </section>
  )
}
