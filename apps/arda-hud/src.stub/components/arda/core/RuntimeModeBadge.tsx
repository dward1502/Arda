// sigil: REPAIR
import type { ArdaRuntimeModeStatus } from '../../../lib/ardaRuntimeMode'

interface RuntimeModeBadgeProps {
  status: ArdaRuntimeModeStatus
}

export default function RuntimeModeBadge({ status }: RuntimeModeBadgeProps) {
  return (
    <span
      className={`runtime-mode-badge runtime-mode-badge--${status.mode}`}
      title={`${status.detail} Data source: ${status.dataSource}.`}
    >
      {status.label}
    </span>
  )
}
