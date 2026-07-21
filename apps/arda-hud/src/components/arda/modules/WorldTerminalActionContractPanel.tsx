// sigil: REPAIR
import { Play, ShieldCheck, TerminalSquare } from 'lucide-react'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../lib/systemActionBus'

export interface WorldTerminalActionContract {
  terminalId: 'terminal_queue' | 'terminal_tools' | 'terminal_status'
  title: string
  sourceZoneId: string
  operatorQuestion: string
  safeActionIds: SystemActionId[]
  governedActionIds: SystemActionId[]
  evidencePaths: string[]
}

export interface WorldTerminalActionDetail {
  actionId: SystemActionId
  label: string
  riskLevel: string
  currentStatus: string
  scheduleState: string
  nextRun: string
  receiptPath: string
  resultPath: string
  governanceGate: string
  relatedEvidence: string[]
  governed: boolean
}

interface WorldTerminalActionContractPanelProps {
  actionDescriptors: SystemActionDescriptor[]
  capabilityStatuses: SystemActionCapabilityStatus[]
  busyActionId?: SystemActionId | null
  message?: string | null
  onRunAction?: (actionId: SystemActionId) => void
}

const WORLD_TERMINAL_ACTION_CONTRACTS: WorldTerminalActionContract[] = [
  {
    terminalId: 'terminal_queue',
    title: 'Queue Terminal',
    sourceZoneId: 'planning_and_queue',
    operatorQuestion: 'What queue pressure can I inspect before recording or changing work?',
    safeActionIds: ['arda.queue_preview_cleanup'],
    governedActionIds: ['arda.queue_capture_pivot'],
    evidencePaths: ['core/projects/tasks/queue.jsonl', 'core/state/queue_summary.json'],
  },
  {
    terminalId: 'terminal_tools',
    title: 'Tools Terminal',
    sourceZoneId: 'systems_health',
    operatorQuestion: 'Which local tool/readiness checks can refresh evidence without repair mutation?',
    safeActionIds: ['arda.setup_run_readiness_check', 'arda.audit_run_repeated_audit'],
    governedActionIds: ['arda.setup_run_repair_flow'],
    evidencePaths: ['core/state/setup_console_readiness.json', 'core/state/repeated_audit_status.json'],
  },
  {
    terminalId: 'terminal_status',
    title: 'Status Terminal',
    sourceZoneId: 'sovereign_world',
    operatorQuestion: 'Which runtime/provider status projections can refresh without changing system state?',
    safeActionIds: ['arda.chronos_run_provider_checks', 'arda.manwe_refresh_provider_intelligence'],
    governedActionIds: [],
    evidencePaths: ['core/state/chronos_runtime.json', 'core/state/provider_intelligence.json'],
  },
]

function descriptorById(descriptors: SystemActionDescriptor[], actionId: SystemActionId): SystemActionDescriptor | null {
  return descriptors.find((descriptor) => descriptor.id === actionId) ?? null
}

function statusById(statuses: SystemActionCapabilityStatus[], actionId: SystemActionId): SystemActionCapabilityStatus | null {
  return statuses.find((status) => status.id === actionId) ?? null
}

export function buildWorldTerminalActionContracts(): WorldTerminalActionContract[] {
  return WORLD_TERMINAL_ACTION_CONTRACTS.map((contract) => ({
    ...contract,
    safeActionIds: [...contract.safeActionIds],
    governedActionIds: [...contract.governedActionIds],
    evidencePaths: [...contract.evidencePaths],
  }))
}

export function buildWorldTerminalActionDetails(
  contract: WorldTerminalActionContract,
  descriptors: SystemActionDescriptor[],
  statuses: SystemActionCapabilityStatus[],
): WorldTerminalActionDetail[] {
  return [...contract.safeActionIds, ...contract.governedActionIds].map((actionId) => {
    const descriptor = descriptorById(descriptors, actionId)
    const status = statusById(statuses, actionId)
    return {
      actionId,
      label: descriptor?.label ?? status?.name ?? actionId,
      riskLevel: descriptor?.riskLevel ?? status?.requiredPermissions ?? 'unknown',
      currentStatus: status?.currentStatus ?? 'not_observed',
      scheduleState: descriptor?.scheduleState ?? status?.scheduleState ?? 'not_scheduled',
      nextRun: status?.nextRun ?? descriptor?.nextRun ?? 'not declared',
      receiptPath: status?.receiptPath ?? descriptor?.receiptPath ?? 'not declared',
      resultPath: status?.resultPath ?? descriptor?.resultPath ?? 'not declared',
      governanceGate: descriptor?.governanceGate ?? status?.governanceGate ?? 'not declared',
      relatedEvidence: descriptor?.relatedEvidence ?? status?.relatedEvidence ?? [],
      governed: contract.governedActionIds.includes(actionId),
    }
  })
}

export default function WorldTerminalActionContractPanel({
  actionDescriptors,
  capabilityStatuses,
  busyActionId,
  message,
  onRunAction,
}: WorldTerminalActionContractPanelProps) {
  const contracts = buildWorldTerminalActionContracts()

  return (
    <section className="planning-action-contract world-terminal-action-contract" aria-label="World terminal action contracts">
      <div className="module-subtitle"><TerminalSquare size={14} /> World Terminal Action Contracts</div>
      <div className="split-stack">
        {contracts.map((contract) => (
          <article className="document-list__item" key={contract.terminalId}>
            {(() => {
              const actionDetails = buildWorldTerminalActionDetails(contract, actionDescriptors, capabilityStatuses)
              return (
                <>
            <div className="document-list__title-row">
              <strong>{contract.title}</strong>
              <span>{contract.sourceZoneId}</span>
            </div>
            <p>{contract.operatorQuestion}</p>
            <dl className="operating-surface-capability-contract">
              <div><dt>Terminal</dt><dd>{contract.terminalId}</dd></div>
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
            <div className="document-list compact" style={{ marginTop: 12 }}>
              {actionDetails.map((detail) => (
                <article className="document-list__item" key={`${contract.terminalId}-${detail.actionId}`}>
                  <div className="document-list__title-row">
                    <strong>{detail.label}</strong>
                    <span>{detail.currentStatus} / {detail.riskLevel}</span>
                  </div>
                  <dl className="operating-surface-capability-contract">
                    <div><dt>Schedule</dt><dd>{detail.scheduleState}</dd></div>
                    <div><dt>Next</dt><dd>{detail.nextRun}</dd></div>
                    <div><dt>Receipt</dt><dd>{detail.receiptPath}</dd></div>
                    <div><dt>Result</dt><dd>{detail.resultPath}</dd></div>
                    <div><dt>Gate</dt><dd>{detail.governanceGate}</dd></div>
                    <div><dt>Evidence</dt><dd>{detail.relatedEvidence.join(', ') || contract.evidencePaths.join(', ')}</dd></div>
                  </dl>
                </article>
              ))}
            </div>
                </>
              )
            })()}
          </article>
        ))}
      </div>
      {message ? <p className="planning-action-contract__message">{message}</p> : null}
    </section>
  )
}
