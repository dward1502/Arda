// sigil: REPAIR
import { CheckCircle2, FolderKanban, ListChecks, ShieldCheck, XCircle } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import StatusBadge from '../../kit/StatusBadge'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import type { HumanAugmentationApproval } from './ArandurApprovalWorkstation'
import { DataFreshnessBadge } from './DataFreshnessBadge'
import { SourceRefreshAffordance } from './SourceRefreshAffordance'

export type ReviewGateKind = 'queue_write' | 'recommendation' | 'mission_approval' | 'hades_lifecycle' | 'athena_policy_readiness'
export type ReviewGateDecisionStatus = 'approved' | 'rejected'

export interface ReviewGateItem {
  id: string
  kind: ReviewGateKind
  title: string
  source: string
  status: string
  decisionClass: string
  evidence: string
  summary: string
  checklist: string[]
  createdAtUtc?: string
}

export interface ReviewGateDecisionRecordPreview {
  decisionClass: string
  commandSignature: string
  approvers: string
  evidence: string
  approvalNote: string
  rejectionNote: string
}

interface ReviewGateWorkstationProps {
  items: ReviewGateItem[]
  approvals: HumanAugmentationApproval[]
  busy: boolean
  message?: string | null
  sourceProvenance?: ArdaSourceProvenance[]
  decisionApprovers?: string
  onApprove: (item: ReviewGateItem) => void
  onReject: (item: ReviewGateItem) => void
}

function statusState(status: string): 'nominal' | 'warning' | 'critical' | 'info' {
  const normalized = status.toLowerCase()
  if (normalized.includes('approved')) return 'nominal'
  if (normalized.includes('reject') || normalized.includes('failed')) return 'critical'
  if (normalized.includes('pending') || normalized.includes('review') || normalized.includes('write')) return 'warning'
  return 'info'
}

function kindLabel(kind: ReviewGateKind): string {
  switch (kind) {
    case 'queue_write':
      return 'Queue Write'
    case 'recommendation':
      return 'Recommendation'
    case 'mission_approval':
      return 'Mission Approval'
    case 'hades_lifecycle':
      return 'HADES Review'
    case 'athena_policy_readiness':
      return 'ATHENA Policy'
  }
}

function latestApprovalFor(item: ReviewGateItem, approvals: HumanAugmentationApproval[]): HumanAugmentationApproval | null {
  return approvals.find((approval) => approval.commandSignature === item.id && approval.decisionClass === item.decisionClass) ?? null
}

function provenanceForReviewItem(item: ReviewGateItem | null, records: ArdaSourceProvenance[] | undefined): ArdaSourceProvenance[] {
  if (!item || !records?.length) return []
  const needles = [item.source, item.evidence, item.kind, item.decisionClass].map((value) => value.toLowerCase())
  return records.filter((record) => {
    const haystack = [record.domainId, record.label, ...record.sourcePaths].join(' ').toLowerCase()
    return needles.some((needle) => needle.length > 0 && haystack.includes(needle))
  })
}

export function buildReviewGateDecisionRecordPreview(
  item: ReviewGateItem,
  approvers = 'aurelius, bacon',
): ReviewGateDecisionRecordPreview {
  const evidence = item.evidence ? `arda-hud,${item.evidence}` : 'arda-hud'
  return {
    decisionClass: item.decisionClass,
    commandSignature: item.id,
    approvers,
    evidence,
    approvalNote: `Approved ${item.decisionClass} ${item.id}: ${item.title}`,
    rejectionNote: `Rejected ${item.decisionClass} ${item.id}: ${item.title}`,
  }
}

export default function ReviewGateWorkstation({
  items,
  approvals,
  busy,
  message,
  sourceProvenance,
  decisionApprovers,
  onApprove,
  onReject,
}: ReviewGateWorkstationProps) {
  const selectedItem = items.find((item) => {
    const approval = latestApprovalFor(item, approvals)
    const status = approval?.status ?? item.status
    return !status.toLowerCase().includes('approved') && !status.toLowerCase().includes('reject')
  }) ?? items[0] ?? null
  const selectedApproval = selectedItem ? latestApprovalFor(selectedItem, approvals) : null
  const provenanceRecords = provenanceForReviewItem(selectedItem, sourceProvenance)
  const decisionRecord = selectedItem ? buildReviewGateDecisionRecordPreview(selectedItem, decisionApprovers) : null
  const visibleItems = items.slice(0, 10)
  const pendingCount = items.filter((item) => !latestApprovalFor(item, approvals)).length

  return (
    <ModuleCard title="Review Gate" eyebrow="Operator approval queue" accent="ember" tag={`${pendingCount} pending`}>
      <div className="split-stack">
        <div>
          <div className="module-subtitle"><ListChecks size={14} /> Pending Review</div>
          <div className="document-list compact">
            {visibleItems.length > 0 ? visibleItems.map((item) => {
              const approval = latestApprovalFor(item, approvals)
              const effectiveStatus = approval?.status ?? item.status
              return (
                <article className="document-list__item" key={`${item.kind}:${item.id}`}>
                  <strong>{item.title}</strong>
                  <span>{kindLabel(item.kind)} / {item.source}</span>
                  <p>{item.summary}</p>
                  <StatusBadge state={statusState(effectiveStatus)} label={effectiveStatus} />
                </article>
              )
            }) : (
              <article className="document-list__item">
                <strong>No review packets detected</strong>
                <p>ARDA did not find ATHENA, Arandur, mission, queue-write, or HADES review records in the current bundle.</p>
              </article>
            )}
          </div>
        </div>
        <div>
          <div className="module-subtitle"><ShieldCheck size={14} /> Selected Packet</div>
          {selectedItem ? (
            <div className="document-list compact">
              <article className="review-gate-steps" aria-label="Review gate workflow">
                <span className="review-gate-steps__item review-gate-steps__item--active">Inspect</span>
                <span className={`review-gate-steps__item ${selectedApproval ? 'review-gate-steps__item--done' : 'review-gate-steps__item--active'}`}>Decide</span>
                <span className={`review-gate-steps__item ${selectedApproval ? 'review-gate-steps__item--done' : ''}`}>Record</span>
              </article>
              <article className="document-list__item">
                <strong>Action requested</strong>
                <span>{kindLabel(selectedItem.kind)} / {selectedItem.source}</span>
                <p>{selectedItem.title}</p>
                <StatusBadge state={statusState(selectedApproval?.status ?? selectedItem.status)} label={selectedApproval?.status ?? selectedItem.status} />
              </article>
              <article className="document-list__item">
                <strong>Why this needs you</strong>
                <span>{selectedItem.decisionClass}</span>
                <p>{selectedItem.summary}</p>
              </article>
              <article className="document-list__item">
                <strong>Decision effect</strong>
                <span>{selectedItem.id}</span>
                <p>Approve only records an operator decision for this packet. Reject records a denial and keeps the gated work blocked until a new packet is prepared.</p>
              </article>
              <article className="document-list__item">
                <strong>Evidence</strong>
                <span>{selectedItem.evidence}</span>
                <p>{selectedItem.createdAtUtc ?? 'no timestamp recorded'}</p>
              </article>
              {provenanceRecords.length > 0 ? (
                <article className="document-list__item">
                  <strong>Source Freshness</strong>
                  <div className="source-provenance-mini-list">
                    {provenanceRecords.slice(0, 3).map((record) => (
                      <span key={record.domainId}>
                        {record.label}
                        <SourceRefreshAffordance record={record} compact />
                        <DataFreshnessBadge record={record} compact />
                      </span>
                    ))}
                  </div>
                </article>
              ) : null}
            </div>
          ) : (
            <article className="document-list__item">
              <strong>No packet selected</strong>
              <p>Review records will appear here when ARDA can read the backing ledgers.</p>
            </article>
          )}
        </div>
      </div>
      <div className="split-stack" style={{ marginTop: 16 }}>
        <div>
          <div className="module-subtitle"><FolderKanban size={14} /> Checklist</div>
          <div className="document-list compact">
            {(selectedItem?.checklist ?? []).length > 0 ? selectedItem?.checklist.map((item) => (
              <article className="document-list__item" key={item}>
                <p>{item}</p>
              </article>
            )) : (
              <article className="document-list__item">
                <p>No checklist was recorded for this packet.</p>
              </article>
            )}
          </div>
        </div>
        <div>
          <div className="module-subtitle"><CheckCircle2 size={14} /> Decision</div>
          <div style={{ display: 'grid', gap: 10 }}>
            {decisionRecord ? (
              <article className="document-list__item review-gate-record-preview">
                <strong>Decision record preview</strong>
                <dl>
                  <div>
                    <dt>decision_class</dt>
                    <dd>{decisionRecord.decisionClass}</dd>
                  </div>
                  <div>
                    <dt>command_signature</dt>
                    <dd>{decisionRecord.commandSignature}</dd>
                  </div>
                  <div>
                    <dt>approvers</dt>
                    <dd>{decisionRecord.approvers}</dd>
                  </div>
                  <div>
                    <dt>evidence</dt>
                    <dd>{decisionRecord.evidence}</dd>
                  </div>
                </dl>
                <p>{decisionRecord.approvalNote}</p>
              </article>
            ) : null}
            <button
              onClick={() => selectedItem ? onApprove(selectedItem) : undefined}
              disabled={!selectedItem || busy}
              className="rounded border border-[#00ff9f66] bg-[#00ff9f1a] px-3 py-2 text-sm font-semibold text-[#00ff9f] transition-colors hover:bg-[#00ff9f2a] disabled:cursor-not-allowed disabled:opacity-60"
            >
              {busy ? 'Recording...' : 'Approve'}
            </button>
            <button
              onClick={() => selectedItem ? onReject(selectedItem) : undefined}
              disabled={!selectedItem || busy}
              className="rounded border border-[#ff333366] bg-[#ff33331a] px-3 py-2 text-sm font-semibold text-[#ff6666] transition-colors hover:bg-[#ff33332a] disabled:cursor-not-allowed disabled:opacity-60"
            >
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                <XCircle size={14} /> Reject
              </span>
            </button>
            {selectedApproval ? (
              <article className="document-list__item">
                <strong>Last decision</strong>
                <span>{selectedApproval.approvers}</span>
                <p>{selectedApproval.note}</p>
              </article>
            ) : null}
            {message ? <div className="text-[11px] text-[#b8c4d4]">{message}</div> : null}
          </div>
        </div>
      </div>
    </ModuleCard>
  )
}
