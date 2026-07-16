import type { ArdaFreshnessState, ArdaSourceProvenance } from '../../../../lib/ardaProvenance'
import { DataFreshnessBadge } from '../DataFreshnessBadge'
import { SourceRefreshAffordance } from '../SourceRefreshAffordance'

interface SourceTrustPanelProps {
  records?: ArdaSourceProvenance[] | null
  limit?: number
}

const STATE_ORDER: ArdaFreshnessState[] = ['fresh', 'stale', 'missing', 'derived', 'blocked', 'unknown']

function countByState(records: ArdaSourceProvenance[]): Record<ArdaFreshnessState, number> {
  return records.reduce<Record<ArdaFreshnessState, number>>(
    (counts, record) => {
      counts[record.state] += 1
      return counts
    },
    { fresh: 0, stale: 0, missing: 0, derived: 0, blocked: 0, unknown: 0 },
  )
}

function priorityForState(state: ArdaFreshnessState): number {
  switch (state) {
    case 'missing':
      return 0
    case 'blocked':
      return 1
    case 'stale':
      return 2
    case 'unknown':
      return 3
    case 'derived':
      return 4
    case 'fresh':
      return 5
    default:
      return 6
  }
}

export default function SourceTrustPanel({ records, limit = 8 }: SourceTrustPanelProps) {
  const sourceRecords = records ?? []
  const counts = countByState(sourceRecords)
  const visibleRecords = [...sourceRecords]
    .sort((left, right) => {
      const statePriority = priorityForState(left.state) - priorityForState(right.state)
      if (statePriority !== 0) return statePriority
      return left.label.localeCompare(right.label)
    })
    .slice(0, limit)

  return (
    <section className="systems-panel source-trust-panel" aria-label="ARDA source trust">
      <div className="systems-panel__header">
        <div>
          <h3>Source Trust</h3>
          <p>Read-only freshness and provenance summary</p>
        </div>
        <strong>{sourceRecords.length}</strong>
      </div>

      {sourceRecords.length === 0 ? (
        <div className="empty-state">No source provenance records available.</div>
      ) : (
        <>
          <div className="token-cloud" aria-label="Source freshness counts">
            {STATE_ORDER.map((state) => (
              <span className="token-chip" key={state}>
                {state}: {counts[state]}
              </span>
            ))}
          </div>
          <div className="path-list">
            {visibleRecords.map((record) => (
              <div className="path-list__item" key={`${record.domainId}:${record.sourcePaths.join('|')}`}>
                <div className="flex items-center justify-between gap-2">
                  <span>{record.label}</span>
                  <span className="source-provenance-list__actions">
                    <SourceRefreshAffordance record={record} compact />
                    <DataFreshnessBadge record={record} compact />
                  </span>
                </div>
                <div className="text-xs text-slate-500">{record.sourcePaths.join(', ') || 'No source paths'}</div>
              </div>
            ))}
          </div>
        </>
      )}
    </section>
  )
}
