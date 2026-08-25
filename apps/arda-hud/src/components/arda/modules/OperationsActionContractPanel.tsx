// sigil: REPAIR
import { Play, ShieldCheck, Wrench } from 'lucide-react'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../lib/systemActionBus'

export interface OperationsActionContract {
  laneId: 'rumil_maintenance' | 'audit_evidence' | 'setup_readiness'
  title: string
  operatorQuestion: string
  safeActionIds: SystemActionId[]
  governedActionIds: SystemActionId[]
  evidencePaths: string[]
}

interface OperationsActionContractPanelProps {
  actionDescriptors: SystemActionDescriptor[]
  capabilityStatuses: SystemActionCapabilityStatus[]
  busyActionId?: SystemActionId | null
  message?: string | null
  onRunAction?: (actionId: SystemActionId) => void
}

const OPERATIONS_ACTION_CONTRACTS: OperationsActionContract[] = [
  {
    laneId: 'rumil_maintenance',
    title: 'Rúmil Maintenance',
    operatorQuestion: 'Which organization and link-check receipts can refresh before any separately approved lifecycle mutation?',
    safeActionIds: ['arda.rumil_preview_organization_plan', 'arda.rumil_run_link_check', 'arda.rumil_run_nightly_organization'],
    governedActionIds: [],
    evidencePaths: ['data/rumil/storage_hygiene_last.json', 'data/rumil/markdown_link_check_last.md'],
  },
  {
    laneId: 'audit_evidence',
    title: 'Audit Evidence',
    operatorQuestion: 'Which audit receipts can refresh without changing source or service state?',
    safeActionIds: ['arda.audit_run_system_audit', 'arda.audit_run_repeated_audit'],
    governedActionIds: [],
    evidencePaths: ['audit/system-audit-runs', 'core/state/repeated_audit_status.json'],
  },
  {
    laneId: 'setup_readiness',
    title: 'Setup Readiness',
    operatorQuestion: 'Which onboarding/setup checks are safe, and which repair flow needs explicit operator approval?',
    safeActionIds: ['arda.setup_run_readiness_check'],
    governedActionIds: ['arda.setup_run_repair_flow'],
    evidencePaths: ['core/state/setup_console_readiness.json', 'core/state/setup_repair_preflight.json'],
  },
]

function descriptorById(descriptors: SystemActionDescriptor[], actionId: SystemActionId): SystemActionDescriptor | null {
  return descriptors.find((descriptor) => descriptor.id === actionId) ?? null
}

function statusById(statuses: SystemActionCapabilityStatus[], actionId: SystemActionId): SystemActionCapabilityStatus | null {
  return statuses.find((status) => status.id === actionId) ?? null
}

export function buildOperationsActionContracts(): OperationsActionContract[] {
  return OPERATIONS_ACTION_CONTRACTS.map((contract) => ({
    ...contract,
    safeActionIds: [...contract.safeActionIds],
    governedActionIds: [...contract.governedActionIds],
    evidencePaths: [...contract.evidencePaths],
  }))
}

export default function OperationsActionContractPanel({
  actionDescriptors,
  capabilityStatuses,
  busyActionId,
  message,
  onRunAction,
}: OperationsActionContractPanelProps) {
  const contracts = buildOperationsActionContracts()

  return (
    <section className="planning-action-contract operations-action-contract" aria-label="Operations action contracts">
      <div className="module-subtitle"><Wrench size={14} /> Operations Action Contracts</div>
      <div className="split-stack">
        {contracts.map((contract) => (
          <article className="document-list__item" key={contract.laneId}>
            <div className="document-list__title-row">
              <strong>{contract.title}</strong>
              <span>{contract.laneId}</span>
            </div>
            <p>{contract.operatorQuestion}</p>
            <dl className="operating-surface-capability-contract">
              <div><dt>Safe actions</dt><dd>{contract.safeActionIds.length}</dd></div>
              <div><dt>Governed</dt><dd>{contract.governedActionIds.length}</dd></div>
              <div><dt>Evidence</dt><dd>{contract.evidencePaths.join(', ')}</dd></div>
            </dl>

            <div className="world-terminal-action-contract__actions">
              {contract.safeActionIds.map((actionId) => {
                const descriptor = descriptorById(actionDescriptors, actionId)
                const status = statusById(capabilityStatuses, actionId)
                const canRun = Boolean(
                  onRunAction &&
                  descriptor &&
                  status?.manualRunAvailable &&
                  descriptor.riskLevel !== 'governed_mutation',
                )
                const busy = busyActionId === actionId
                return (
                  <button
                    type="button"
                    key={actionId}
                    className={`refresh-button ${busy ? 'refresh-button--active' : ''}`}
                    disabled={!canRun || busy}
                    onClick={() => onRunAction?.(actionId)}
                    title={descriptor?.purpose ?? actionId}
                  >
                    <Play size={12} /> {busy ? 'Running...' : descriptor?.label ?? actionId}
                  </button>
                )
              })}
            </div>

            {contract.governedActionIds.length > 0 ? (
              <div className="world-terminal-action-contract__governed">
                {contract.governedActionIds.map((actionId) => {
                  const descriptor = descriptorById(actionDescriptors, actionId)
                  const status = statusById(capabilityStatuses, actionId)
                  return (
                    <div className="planning-action-contract__command" key={actionId}>
                      <ShieldCheck size={14} />
                      <code>{descriptor?.label ?? actionId}: {status?.currentStatus ?? 'blocked'} / {descriptor?.governanceGate ?? 'operator gate required'}</code>
                    </div>
                  )
                })}
              </div>
            ) : null}
          </article>
        ))}
      </div>
      {message ? <p className="planning-action-contract__message">{message}</p> : null}
    </section>
  )
}
