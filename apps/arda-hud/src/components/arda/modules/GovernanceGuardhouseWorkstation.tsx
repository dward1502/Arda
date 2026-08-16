import { useEffect, useMemo, useState } from 'react'
import { CheckCircle2, ShieldCheck, XCircle } from 'lucide-react'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import ModuleCard from '../ModuleCard'
import StatusBadge from '../../kit/StatusBadge'
import type { SourceCoverageBadgeState } from './SourceCoverageBadge'
import type { ArandurActiveTask, HumanAugmentationApproval } from './ArandurApprovalWorkstation'
import {
  buildReviewGateDecisionRecordPreview,
  type ReviewGateItem,
} from './ReviewGateWorkstation'

interface GovernanceSummary {
  ready: boolean
  weights: Array<{ label: string; value: number }>
  thresholds: Array<{ label: string; value: number }>
}

interface AutonomyReadinessSummary {
  posture: string
  checkpoint: Array<{ label: string; value: string }>
  evidence: Array<{ phase: string; title: string; status: string; source: string }>
  nextUnlocks: Array<{ title: string; status: string; requires: string }>
}

export interface GuardhouseSourceState {
  id: 'guardhouse' | 'edge-contract' | 'nightly-doctrine' | 'policy-authority'
  label: string
  path: string
  state: string
  timestamp: string | null
}

interface GovernanceGuardhouseWorkstationProps {
  governance: GovernanceSummary
  governanceSignals: Array<{ label: string; value: string }>
  autonomyReadiness: AutonomyReadinessSummary
  items: ReviewGateItem[]
  activeTasks?: ArandurActiveTask[]
  approvals: HumanAugmentationApproval[]
  sourceProvenance: ArdaSourceProvenance[]
  sourceCoverage?: SourceCoverageBadgeState
  busy: boolean
  message?: string | null
  decisionApprovers?: string
  onApprove: (item: ReviewGateItem) => void
  onReject: (item: ReviewGateItem) => void
  onDefer: (item: ReviewGateItem) => void
  onCancelTask?: (task: ArandurActiveTask) => void
  onRetryTask?: (task: ArandurActiveTask) => void
}

const GUARDHOUSE_SOURCES: Array<Omit<GuardhouseSourceState, 'state' | 'timestamp'>> = [
  { id: 'guardhouse', label: 'Guardhouse', path: 'core/state/warden_guardhouse.json' },
  { id: 'edge-contract', label: 'Edge contract', path: 'core/state/warden_edge_contract.json' },
  { id: 'nightly-doctrine', label: 'Nightly doctrine', path: 'core/state/warden_nightly_doctrine.json' },
  { id: 'policy-authority', label: 'Policy authority', path: 'core/state/warden_policy_authority.json' },
]

const REVIEW_SOURCE_HINTS = [
  'data/arandur/',
  'data/hades/lifecycle_review_queue.jsonl',
  'data/athena/policy_readiness.jsonl',
]

function sourceStateLabel(record: ArdaSourceProvenance | undefined): string {
  if (!record || ['missing', 'blocked', 'unknown'].includes(record.state)) return 'unavailable'
  if (record.state === 'stale') return `stale ${record.sourceKind}`
  if (record.sourceKind === 'live') return 'connected'
  if (record.sourceKind === 'derived' || record.state === 'derived') return 'projected'
  return record.sourceKind
}

export function deriveGuardhouseSourceStates(records: ArdaSourceProvenance[]): GuardhouseSourceState[] {
  return GUARDHOUSE_SOURCES.map((source) => {
    const record = records.find((candidate) => candidate.sourcePaths.some((path) => path === source.path))
    return {
      ...source,
      state: sourceStateLabel(record),
      timestamp: record?.generatedAtUtc ?? record?.observedAtUtc ?? null,
    }
  })
}

function latestApprovalFor(item: ReviewGateItem, approvals: HumanAugmentationApproval[]): HumanAugmentationApproval | null {
  return approvals.find((approval) => (
    approval.commandSignature === item.id && approval.decisionClass === item.decisionClass
  )) ?? null
}

function statusState(status: string): 'nominal' | 'warning' | 'critical' | 'info' {
  const normalized = status.toLowerCase()
  if (normalized.includes('approved')) return 'nominal'
  if (normalized.includes('reject') || normalized.includes('failed')) return 'critical'
  if (normalized.includes('pending') || normalized.includes('review')) return 'warning'
  return 'info'
}

function canDecide(item: ReviewGateItem, approval: HumanAugmentationApproval | null): boolean {
  if (approval) return false
  if (item.decisionClass === 'task_runtime') return false
  if (item.kind === 'athena_policy_readiness') return false
  return !item.status.toLowerCase().includes('reference_only')
}

function hasAvailableReviewSource(records: ArdaSourceProvenance[]): boolean {
  return records.some((record) => (
    record.sourcePaths.some((path) => REVIEW_SOURCE_HINTS.some((hint) => path.includes(hint))) &&
    !['missing', 'blocked', 'unknown'].includes(record.state)
  ))
}

export interface GovernanceGuardhouseViewModel {
  posture: Array<{ label: string; value: string }>
  records: ReviewGateItem[]
  sources: GuardhouseSourceState[]
  recordSourceAvailable: boolean
}

export function deriveGovernanceGuardhouseViewModel({
  governance,
  governanceSignals,
  autonomyReadiness,
  items,
  sourceProvenance,
  sourceCoverage,
}: Pick<GovernanceGuardhouseWorkstationProps,
  'governance' | 'governanceSignals' | 'autonomyReadiness' | 'items' | 'sourceProvenance' | 'sourceCoverage'
>): GovernanceGuardhouseViewModel {
  const sources = deriveGuardhouseSourceStates(sourceProvenance)
  return {
    posture: [
      { label: 'Policy', value: governance.ready ? 'active' : 'unconfigured' },
      { label: 'Autonomy', value: autonomyReadiness.posture },
      { label: 'Guardhouse', value: sources[0]?.state ?? 'unavailable' },
      { label: 'Sources', value: sourceCoverage?.label ?? 'source map unavailable' },
      ...governanceSignals.slice(0, 2),
    ],
    records: items,
    sources,
    recordSourceAvailable: hasAvailableReviewSource(sourceProvenance),
  }
}

export default function GovernanceGuardhouseWorkstation({
  governance,
  governanceSignals,
  autonomyReadiness,
  items,
  activeTasks = [],
  approvals,
  sourceProvenance,
  sourceCoverage,
  busy,
  message,
  decisionApprovers,
  onApprove,
  onReject,
  onDefer,
  onCancelTask,
  onRetryTask,
}: GovernanceGuardhouseWorkstationProps) {
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const taskRecords = useMemo<ReviewGateItem[]>(() => activeTasks.map((task) => ({
    id: `active-task:${task.id}`,
    kind: 'queue_write',
    title: task.title,
    source: 'core/projects/tasks/queue.jsonl',
    status: task.status,
    summary: task.detail ?? task.result ?? `Governed task owned by ${task.owner}`,
    decisionClass: 'task_runtime',
    evidence: task.executionReceiptDigest ?? 'No execution receipt recorded',
    checklist: [
      `owner: ${task.owner}`,
      `priority: ${task.priority}`,
      task.workbenchRunId ? `run: ${task.workbenchRunId}` : 'run: unavailable',
    ],
    createdAtUtc: task.leaseExpiresAtUtc ?? null,
  })), [activeTasks])
  const records = useMemo(() => [...items, ...taskRecords], [items, taskRecords])
  const viewModel = useMemo(() => deriveGovernanceGuardhouseViewModel({
    governance,
    governanceSignals,
    autonomyReadiness,
    items: records,
    sourceProvenance,
    sourceCoverage,
  }), [autonomyReadiness, governance, governanceSignals, records, sourceCoverage, sourceProvenance])
  const selectedItem = records.find((item) => item.id === selectedId) ?? records[0] ?? null
  const selectedTask = selectedItem?.id.startsWith('active-task:')
    ? activeTasks.find((task) => `active-task:${task.id}` === selectedItem.id) ?? null
    : null
  const selectedApproval = selectedItem ? latestApprovalFor(selectedItem, approvals) : null
  const decisionPreview = selectedItem
    ? buildReviewGateDecisionRecordPreview(selectedItem, decisionApprovers)
    : null


  useEffect(() => {
    if (selectedId && !records.some((item) => item.id === selectedId)) setSelectedId(null)
  }, [records, selectedId])

  return (
    <ModuleCard
      title="Governance + Guardhouse"
      eyebrow="Policy posture and governed review"
      accent="ember"
      tag={`${records.length} records`}
    >
      <section className="governance-posture-rail" aria-label="Governance posture rail">
        {viewModel.posture.map((signal) => (
          <span key={signal.label}><strong>{signal.label}</strong>{signal.value}</span>
        ))}
      </section>

      <section className="guardhouse-source-rail" aria-label="Guardhouse source states">
        {viewModel.sources.map((source) => (
          <span key={source.id} className={`guardhouse-source guardhouse-source--${source.state.replace(/ /g, '-')}`}>
            <strong>{source.label}</strong>
            <span>{source.state}</span>
            <small>{source.timestamp ?? 'no source timestamp'}</small>
          </span>
        ))}
      </section>

      <div className="governance-master-detail">
        <nav className="governance-record-index" aria-label="Governance record index">
          <div className="module-subtitle"><ShieldCheck size={14} /> Review records</div>
          {viewModel.records.length > 0 ? viewModel.records.map((item) => {
            const approval = latestApprovalFor(item, approvals)
            const effectiveStatus = approval?.status ?? item.status
            return (
              <button
                type="button"
                key={`${item.kind}:${item.id}`}
                className={selectedItem?.id === item.id ? 'governance-record is-selected' : 'governance-record'}
                aria-pressed={selectedItem?.id === item.id}
                onClick={() => setSelectedId(item.id)}
              >
                <strong>{item.title}</strong>
                <span>{item.source}</span>
                <StatusBadge state={statusState(effectiveStatus)} label={effectiveStatus} />
              </button>
            )
          }) : (
            <div className="governance-empty-state">
              <strong>{viewModel.recordSourceAvailable ? 'No pending governance records' : 'Governance record source unavailable'}</strong>
              <p>{viewModel.recordSourceAvailable
                ? 'The connected review sources contain no records requiring attention.'
                : 'ARDA cannot currently read a recognized Arandur, HADES, or ATHENA review source.'}</p>
            </div>
          )}
        </nav>

        <section className="governance-record-detail" aria-label="Selected governance record">
          {selectedItem ? (
            <>
              <header>
                <span>{selectedItem.kind.replace(/_/g, ' ')}</span>
                <h3>{selectedItem.title}</h3>
                <StatusBadge
                  state={statusState(selectedApproval?.status ?? selectedItem.status)}
                  label={selectedApproval?.status ?? selectedItem.status}
                />
              </header>
              <dl className="governance-record-facts">
                <div><dt>Summary</dt><dd>{selectedItem.summary}</dd></div>
                <div><dt>Authority</dt><dd>{selectedItem.decisionClass}</dd></div>
                <div><dt>Affected scope</dt><dd>{selectedItem.source}</dd></div>
                <div><dt>Evidence</dt><dd>{selectedItem.evidence || 'No evidence reference recorded'}</dd></div>
                <div><dt>Timestamp</dt><dd>{selectedItem.createdAtUtc ?? 'No timestamp recorded'}</dd></div>
                <div><dt>Receipt binding</dt><dd>{decisionPreview?.commandSignature ?? selectedItem.id}</dd></div>
              </dl>
              <details className="governance-progressive-detail">
                <summary>Evidence checklist and receipt preview</summary>
                <ul>
                  {selectedItem.checklist.length > 0
                    ? selectedItem.checklist.map((entry) => <li key={entry}>{entry}</li>)
                    : <li>No checklist was recorded.</li>}
                </ul>
                <p>{decisionPreview?.evidence}</p>
              </details>
              <footer className="governance-action-strip" aria-label="Contextual governance actions">
                {selectedTask && ['in_progress', 'running', 'claimed'].includes(selectedTask.status) && onCancelTask ? (
                  <button type="button" disabled={busy} onClick={() => onCancelTask(selectedTask)}>Cancel run</button>
                ) : selectedTask && selectedTask.status === 'failed' && selectedTask.result !== 'cancelled' && onRetryTask ? (
                  <button type="button" disabled={busy} onClick={() => onRetryTask(selectedTask)}>Retry governed task</button>
                ) : canDecide(selectedItem, selectedApproval) ? (
                  <>
                    <button type="button" disabled={busy} onClick={() => onDefer(selectedItem)}>
                      Defer
                    </button>
                    <button type="button" disabled={busy} onClick={() => onApprove(selectedItem)}>
                      <CheckCircle2 size={14} /> Approve
                    </button>
                    <button type="button" disabled={busy} onClick={() => onReject(selectedItem)}>
                      <XCircle size={14} /> Reject
                    </button>
                  </>
                ) : (
                  <span>{selectedApproval
                    ? `Decision recorded by ${selectedApproval.approvers}`
                    : 'No valid mutation authority is exposed for this record state.'}</span>
                )}
                {message ? <small>{message}</small> : null}
              </footer>
            </>
          ) : (
            <div className="governance-empty-state">
              <strong>No record selected</strong>
              <p>Selectable detail appears when the review source has records.</p>
            </div>
          )}
        </section>
      </div>
    </ModuleCard>
  )
}
