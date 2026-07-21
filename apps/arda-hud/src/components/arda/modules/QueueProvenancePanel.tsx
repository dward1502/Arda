import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import type { JsonRecord } from '../../../lib/ardaSource'
import { DataFreshnessBadge } from './DataFreshnessBadge'
import { SourceRefreshAffordance } from './SourceRefreshAffordance'

interface QueueProvenancePanelProps {
  records?: ArdaSourceProvenance[] | null
  queueFederation?: JsonRecord | null
  limit?: number
}

const asRecord = (value: unknown): JsonRecord | null => (
  value !== null && typeof value === 'object' && !Array.isArray(value) ? value as JsonRecord : null
)

const asArray = (value: unknown): unknown[] => Array.isArray(value) ? value : []

const asString = (value: unknown, fallback: string): string => (
  typeof value === 'string' && value.length > 0 ? value : fallback
)

function isQueueSource(record: ArdaSourceProvenance): boolean {
  const haystack = [record.domainId, record.label, ...record.sourcePaths].join(' ').toLowerCase()
  return (
    haystack.includes('queue') ||
    haystack.includes('planning') ||
    haystack.includes('task_lifecycle') ||
    haystack.includes('operator_actions') ||
    haystack.includes('hades/lifecycle_review')
  )
}

function priorityForRecord(record: ArdaSourceProvenance): number {
  if (record.sourcePaths.some((path) => path.includes('core/projects/tasks/queue.jsonl'))) return 0
  if (record.sourcePaths.some((path) => path.includes('queue_summary'))) return 1
  if (record.sourcePaths.some((path) => path.includes('hades/lifecycle_review'))) return 2
  if (record.sourcePaths.some((path) => path.includes('task_lifecycle'))) return 3
  return 4
}

export default function QueueProvenancePanel({ records, queueFederation, limit = 5 }: QueueProvenancePanelProps) {
  const queueRecords = (records ?? [])
    .filter(isQueueSource)
    .sort((left, right) => {
      const priority = priorityForRecord(left) - priorityForRecord(right)
      if (priority !== 0) return priority
      return left.label.localeCompare(right.label)
    })
    .slice(0, limit)

  const stageContracts = asArray(queueFederation?.sources)
    .map((source) => asRecord(source))
    .filter((source): source is JsonRecord => source !== null)
    .slice(0, 6)

  if (queueRecords.length === 0 && stageContracts.length === 0) {
    return null
  }

  return (
    <section className="queue-provenance-panel" aria-label="Queue source freshness">
      <div className="module-subtitle">Queue Source Freshness</div>
      {stageContracts.length > 0 ? (
        <div className="document-list compact" aria-label="Queue stage contracts">
          {stageContracts.map((source) => {
            const id = asString(source.id, 'unknown lane')
            const recordClass = asString(source.default_record_class, 'evidence')
            const laneSubclass = asString(source.lane_subclass, recordClass)
            const receipt = asString(source.promotion_receipt_required, 'explicit promotion receipt')
            return (
              <article className="document-list__item" key={`stage-contract-${id}`}>
                <div className="document-list__title-row">
                  <strong>{id}</strong>
                  <span>{recordClass}</span>
                </div>
                <p>lane: {laneSubclass}; receipt: {receipt}</p>
              </article>
            )
          })}
        </div>
      ) : null}
      <div className="source-provenance-list">
        {queueRecords.map((record) => (
          <article className="source-provenance-list__item" key={record.domainId}>
            <div>
              <strong>{record.label}</strong>
              <span>{record.sourcePaths.join(', ') || 'No source paths'}</span>
            </div>
            <span className="source-provenance-list__actions">
              <SourceRefreshAffordance record={record} compact />
              <DataFreshnessBadge record={record} compact />
            </span>
          </article>
        ))}
      </div>
    </section>
  )
}
