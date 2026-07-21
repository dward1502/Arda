// sigil: REPAIR
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { DataFreshnessBadge } from './DataFreshnessBadge'

interface SourceFreshnessStripProps {
  records?: ArdaSourceProvenance[]
  terms: string[]
  title: string
  limit?: number
}

function matchesRecord(record: ArdaSourceProvenance, terms: string[]): boolean {
  const haystack = [record.domainId, record.label, ...record.sourcePaths, ...(record.derivedFrom ?? [])]
    .join(' ')
    .toLowerCase()
  return terms.some((term) => haystack.includes(term.toLowerCase()))
}

export default function SourceFreshnessStrip({
  records,
  terms,
  title,
  limit = 3,
}: SourceFreshnessStripProps) {
  const visibleRecords = (records ?? []).filter((record) => matchesRecord(record, terms)).slice(0, limit)
  if (visibleRecords.length === 0) {
    return null
  }

  return (
    <div className="source-freshness-strip" aria-label={title}>
      <div className="module-subtitle">{title}</div>
      <div className="source-provenance-list compact">
        {visibleRecords.map((record) => (
          <article className="source-provenance-list__item" key={record.domainId}>
            <div>
              <strong>{record.label}</strong>
              <span>{record.generatedAtUtc ?? record.observedAtUtc ?? 'timestamp unknown'}</span>
            </div>
            <span className="source-provenance-list__actions">
              <DataFreshnessBadge record={record} compact />
            </span>
          </article>
        ))}
      </div>
    </div>
  )
}
