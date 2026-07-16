// sigil: REPAIR
import { FolderKanban, ShieldCheck, XCircle } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import StatusBadge from '../../kit/StatusBadge'

export interface HumanAugmentationApproval {
  id: string
  decisionClass: string
  approvers: string
  status: string
  note: string
  commandSignature?: string | null
}

export interface ArandurQueueWriteRequest {
  id: string
  missionCandidateId: string
  queueProposalId: string
  title: string
  scope: string
  justification: string
  createdAtUtc: string
  canonicalQueueSha1: string
  proposalSha1: string
  reviewRequired: boolean
  reviewChecklist: string[]
  requiresFutureHumanApproval: boolean
  requiresSeparateFutureCanonicalQueueWrite: boolean
  mutationPolicy: Record<string, string>
  writePending: boolean
  executionStatus: string
  canonicalQueueTaskId?: string | null
}

interface ArandurApprovalWorkstationProps {
  approvals: HumanAugmentationApproval[]
  queueWriteRequests: ArandurQueueWriteRequest[]
  busy: boolean
  message?: string | null
  onApprove: (request: ArandurQueueWriteRequest) => void
  onReject: (request: ArandurQueueWriteRequest) => void
}

function statusState(status: string): 'nominal' | 'warning' | 'critical' | 'info' {
  const normalized = status.toLowerCase()
  if (normalized.includes('approved')) return 'nominal'
  if (normalized.includes('reject') || normalized.includes('failed')) return 'critical'
  if (normalized.includes('pending') || normalized.includes('review')) return 'warning'
  return 'info'
}

function shortSha(value: string): string {
  return value.length > 12 ? value.slice(0, 12) : value
}

export default function ArandurApprovalWorkstation({
  approvals,
  queueWriteRequests,
  busy,
  message,
  onApprove,
  onReject,
}: ArandurApprovalWorkstationProps) {
  const selectedRequest = queueWriteRequests.find((request) => request.executionStatus !== 'executed') ?? queueWriteRequests[0] ?? null
  const matchingApproval = selectedRequest
    ? approvals.find((approval) => approval.commandSignature === selectedRequest.id)
    : approvals[0]

  return (
    <ModuleCard title="Arandur Approval Workstation" eyebrow="Operator-gated automation" accent="ember" tag="Arandur">
      <div className="split-stack">
        <div>
          <div className="module-subtitle"><FolderKanban size={14} /> Queue Write Request</div>
          {selectedRequest ? (
            <div className="document-list compact">
              <article className="document-list__item">
                <strong>{selectedRequest.title}</strong>
                <span>{selectedRequest.id}</span>
                <p>{selectedRequest.justification}</p>
                <StatusBadge state={statusState(selectedRequest.executionStatus)} label={selectedRequest.executionStatus} />
              </article>
              <article className="document-list__item">
                <strong>{selectedRequest.missionCandidateId}</strong>
                <span>{selectedRequest.scope}</span>
                <p>proposal {selectedRequest.queueProposalId}</p>
              </article>
              <article className="document-list__item">
                <strong>Fingerprints</strong>
                <span>queue {shortSha(selectedRequest.canonicalQueueSha1)} / proposal {shortSha(selectedRequest.proposalSha1)}</span>
                <p>{selectedRequest.canonicalQueueTaskId ? `canonical task ${selectedRequest.canonicalQueueTaskId}` : selectedRequest.createdAtUtc}</p>
              </article>
            </div>
          ) : (
            <article className="document-list__item">
              <strong>No Arandur queue write requests detected</strong>
              <p>Generate a Phase 6H queue write request before approving canonical task queue mutation.</p>
            </article>
          )}
        </div>
        <div>
          <div className="module-subtitle"><ShieldCheck size={14} /> Safety Gates</div>
          <div className="token-cloud">
            {selectedRequest?.writePending ? <span className="token-chip">write pending</span> : <span className="token-chip">write not pending</span>}
            {selectedRequest?.reviewRequired ? <span className="token-chip">review required</span> : <span className="token-chip">review not flagged</span>}
            {selectedRequest?.requiresFutureHumanApproval ? <span className="token-chip">requires future human approval</span> : null}
            {selectedRequest?.requiresSeparateFutureCanonicalQueueWrite ? <span className="token-chip">requires separate future canonical queue write</span> : null}
          </div>
          <div className="document-list compact" style={{ marginTop: 12 }}>
            {(selectedRequest?.reviewChecklist ?? []).map((item) => (
              <article className="document-list__item" key={item}>
                <p>{item}</p>
              </article>
            ))}
            {selectedRequest ? Object.entries(selectedRequest.mutationPolicy).map(([key, value]) => (
              <article className="document-list__item" key={key}>
                <p>{key}: {value}</p>
              </article>
            )) : null}
          </div>
        </div>
      </div>
      <div className="split-stack" style={{ marginTop: 16 }}>
        <div>
          <div className="module-subtitle"><ShieldCheck size={14} /> Approval State</div>
          {matchingApproval ? (
            <article className="document-list__item">
              <strong>{matchingApproval.decisionClass}</strong>
              <span>{matchingApproval.approvers}</span>
              <p>{matchingApproval.note}</p>
              <StatusBadge state={statusState(matchingApproval.status)} label={matchingApproval.status} />
            </article>
          ) : (
            <article className="document-list__item">
              <strong>No matching approval recorded</strong>
              <p>Use the controls to record an operator decision against this request signature.</p>
            </article>
          )}
        </div>
        <div>
          <div className="module-subtitle"><XCircle size={14} /> Decision Controls</div>
          <div style={{ display: 'grid', gap: 10 }}>
            <button
              onClick={() => selectedRequest ? onApprove(selectedRequest) : undefined}
              disabled={!selectedRequest || busy}
              className="rounded border border-[#00ff9f66] bg-[#00ff9f1a] px-3 py-2 text-sm font-semibold text-[#00ff9f] transition-colors hover:bg-[#00ff9f2a] disabled:cursor-not-allowed disabled:opacity-60"
            >
              {busy ? 'Recording...' : 'Approve queue write'}
            </button>
            <button
              onClick={() => selectedRequest ? onReject(selectedRequest) : undefined}
              disabled={!selectedRequest || busy}
              className="rounded border border-[#ff333366] bg-[#ff33331a] px-3 py-2 text-sm font-semibold text-[#ff6666] transition-colors hover:bg-[#ff33332a] disabled:cursor-not-allowed disabled:opacity-60"
            >
              Reject queue write
            </button>
            {message ? <div className="text-[11px] text-[#b8c4d4]">{message}</div> : null}
          </div>
        </div>
      </div>
    </ModuleCard>
  )
}
