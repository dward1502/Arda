import React from 'react'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'

interface DataFreshnessBadgeProps {
  record: ArdaSourceProvenance
  compact?: boolean
  onDetailsClick?: () => void
  className?: string
}

export const DataFreshnessBadge: React.FC<DataFreshnessBadgeProps> = ({
  record,
  compact = false,
  onDetailsClick,
  className = '',
}) => {
  const getDisplayState = () => {
    const states = {
      fresh: { badge: '🟢', label: 'Fresh' },
      stale: { badge: '🟡', label: 'Stale' },
      missing: { badge: '🔴', label: 'Missing' },
      derived: { badge: '🟣', label: 'Derived' },
      blocked: { badge: '🟤', label: 'Blocked' },
      unknown: { badge: '⚪', label: 'Unknown' },
    } satisfies Record<ArdaSourceProvenance['state'], { badge: string; label: string }>

    if (compact) {
      return states[record.state]
    }

    return states[record.state]
  }

  const { badge, label } = getDisplayState()
  const hasSources = record.sourcePaths.length > 0
  const tooltipText = hasSources
    ? `Source: ${record.sourcePaths.join(', ')}\nState: ${record.state}`
    : `No sources available\nState: ${record.state}`

  return (
    <div
      className={`inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium ${className}`}
      title={tooltipText}
    >
      <span className="mr-1 text-lg leading-none">{badge}</span>
      <span>{label}</span>
      {!compact && hasSources && onDetailsClick && (
        <button
          onClick={onDetailsClick}
          className="ml-2 text-xs underline"
          aria-label="View source details"
        >
          Details
        </button>
      )}
    </div>
  )
}
