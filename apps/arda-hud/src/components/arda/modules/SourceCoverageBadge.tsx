// sigil: REPAIR

export interface SourceCoverageBadgeState {
  status: 'backed' | 'partial' | 'unmapped'
  label: string
  missingCount: number
}

interface SourceCoverageBadgeProps {
  coverage?: SourceCoverageBadgeState | null
}

export default function SourceCoverageBadge({ coverage }: SourceCoverageBadgeProps) {
  if (!coverage) return null

  const toneClass = `source-coverage-badge source-coverage-badge--${coverage.status}`
  const missingLabel = coverage.missingCount === 1 ? '1 missing' : `${coverage.missingCount} missing`

  return (
    <span className={toneClass} title={coverage.label}>
      <span>{coverage.label}</span>
      <strong>{missingLabel}</strong>
    </span>
  )
}
