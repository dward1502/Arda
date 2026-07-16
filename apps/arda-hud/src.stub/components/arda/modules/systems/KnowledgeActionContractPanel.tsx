// sigil: REPAIR
import { BookOpenCheck, Play, ShieldCheck } from 'lucide-react'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../../lib/systemActionBus'

interface KnowledgeActionContractPanelProps {
  actionDescriptors: SystemActionDescriptor[]
  capabilityStatuses: SystemActionCapabilityStatus[]
  policySummary?: {
    status: string
    policyReadyTotal: number
    referenceOnlyTotal: number
    reviewPressureTotal: number
    nextOperatorAction: string
    promotionPreviewAvailable: boolean
    governanceGate: string
  }
  busyActionId?: SystemActionId | null
  message?: string | null
  onRunAction?: (actionId: SystemActionId) => void
}

const SAFE_KNOWLEDGE_ACTION_IDS: SystemActionId[] = ['arda.athena_refresh_digest']
const GOVERNED_KNOWLEDGE_ACTION_IDS: SystemActionId[] = ['arda.athena_ingest_knowledge', 'arda.athena_promote_policy_ready']

function descriptorById(descriptors: SystemActionDescriptor[], actionId: SystemActionId): SystemActionDescriptor | null {
  return descriptors.find((descriptor) => descriptor.id === actionId) ?? null
}

function statusById(statuses: SystemActionCapabilityStatus[], actionId: SystemActionId): SystemActionCapabilityStatus | null {
  return statuses.find((status) => status.id === actionId) ?? null
}

export function buildKnowledgeActionContract() {
  return {
    title: 'Knowledge Action Contract',
    sourceZoneId: 'knowledge_and_reasoning',
    operatorQuestion: 'What ATHENA evidence can refresh safely, and which knowledge mutations require human review?',
    safeActionIds: [...SAFE_KNOWLEDGE_ACTION_IDS],
    governedActionIds: [...GOVERNED_KNOWLEDGE_ACTION_IDS],
    evidencePaths: [
      'core/state/athena_runtime.json',
      'core/state/knowledge_triage_registry.jsonl',
      'data/athena/digest.jsonl',
      'data/athena/policy_readiness.jsonl',
    ],
  }
}

export default function KnowledgeActionContractPanel({
  actionDescriptors,
  capabilityStatuses,
  policySummary,
  busyActionId,
  message,
  onRunAction,
}: KnowledgeActionContractPanelProps) {
  const contract = buildKnowledgeActionContract()
  const pressure = policySummary ?? {
    status: 'unknown',
    policyReadyTotal: 0,
    referenceOnlyTotal: 0,
    reviewPressureTotal: 0,
    nextOperatorAction: 'refresh_athena_digest',
    promotionPreviewAvailable: false,
    governanceGate: 'human_review_required',
  }

  return (
    <section className="planning-action-contract knowledge-action-contract" aria-label="Knowledge action contract">
      <div className="module-subtitle"><BookOpenCheck size={14} /> {contract.title}</div>
      <article className="document-list__item">
        <div className="document-list__title-row">
          <strong>{contract.sourceZoneId}</strong>
          <span>{contract.safeActionIds.length} safe / {contract.governedActionIds.length} governed</span>
        </div>
        <p>{contract.operatorQuestion}</p>
        <dl className="operating-surface-capability-contract">
          <div><dt>Evidence</dt><dd>{contract.evidencePaths.join(', ')}</dd></div>
          <div><dt>Safe lane</dt><dd>projection refresh only</dd></div>
          <div><dt>Governed lane</dt><dd>human review required</dd></div>
          <div><dt>Policy state</dt><dd>{pressure.status}</dd></div>
          <div><dt>Review pressure</dt><dd>{pressure.reviewPressureTotal} blocked / {pressure.policyReadyTotal} ready / {pressure.referenceOnlyTotal} reference-only</dd></div>
          <div><dt>Next action</dt><dd>{pressure.nextOperatorAction}</dd></div>
        </dl>

        <div className="world-terminal-action-contract__actions">
          {contract.safeActionIds.map((actionId) => {
            const descriptor = descriptorById(actionDescriptors, actionId)
            const status = statusById(capabilityStatuses, actionId)
            const busy = busyActionId === actionId
            const canRun = Boolean(onRunAction && descriptor && status?.manualRunAvailable && descriptor.riskLevel !== 'governed_mutation')
            return (
              <button
                type="button"
                className={`refresh-button ${busy ? 'refresh-button--active' : ''}`}
                disabled={!canRun || busy}
                key={actionId}
                onClick={() => onRunAction?.(actionId)}
                title={descriptor?.purpose ?? actionId}
              >
                <Play size={12} /> {busy ? 'Running...' : descriptor?.label ?? actionId}
              </button>
            )
          })}
        </div>

        <div className="world-terminal-action-contract__governed">
          {contract.governedActionIds.map((actionId) => {
            const descriptor = descriptorById(actionDescriptors, actionId)
            const status = statusById(capabilityStatuses, actionId)
            const busy = busyActionId === actionId
            const canPreview = Boolean(
              onRunAction
                && actionId === 'arda.athena_promote_policy_ready'
                && descriptor
                && status?.manualRunAvailable
                && status?.dryRunAvailable,
            )
            return (
              <div className="planning-action-contract__command" key={actionId}>
                <ShieldCheck size={14} />
                <code>{descriptor?.label ?? actionId}: {status?.currentStatus ?? 'blocked'} / {descriptor?.governanceGate ?? 'human review required'}</code>
                {canPreview ? (
                  <button
                    type="button"
                    className={`refresh-button ${busy ? 'refresh-button--active' : ''}`}
                    disabled={busy}
                    onClick={() => onRunAction?.(actionId)}
                    title={descriptor?.purpose ?? actionId}
                  >
                    <Play size={12} /> {busy ? 'Running...' : `Preview ${descriptor?.label ?? actionId}`}
                  </button>
                ) : null}
              </div>
            )
          })}
        </div>
      </article>
      {message ? <p className="planning-action-contract__message">{message}</p> : null}
    </section>
  )
}
