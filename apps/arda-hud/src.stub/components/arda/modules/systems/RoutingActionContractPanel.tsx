// sigil: REPAIR
import { Network, Play, ShieldCheck } from 'lucide-react'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../../lib/systemActionBus'

interface RoutingActionContractPanelProps {
  actionDescriptors: SystemActionDescriptor[]
  capabilityStatuses: SystemActionCapabilityStatus[]
  busyActionId?: SystemActionId | null
  message?: string | null
  onRunAction?: (actionId: SystemActionId) => void
}

const SAFE_ROUTING_ACTION_IDS: SystemActionId[] = [
  'arda.chronos_run_provider_checks',
  'arda.charon_refresh_provider_intelligence',
]

function descriptorById(descriptors: SystemActionDescriptor[], actionId: SystemActionId): SystemActionDescriptor | null {
  return descriptors.find((descriptor) => descriptor.id === actionId) ?? null
}

function statusById(statuses: SystemActionCapabilityStatus[], actionId: SystemActionId): SystemActionCapabilityStatus | null {
  return statuses.find((status) => status.id === actionId) ?? null
}

export function buildRoutingActionContract() {
  return {
    title: 'Routing Action Contract',
    sourceZoneId: 'routing_and_providers',
    operatorQuestion: 'Which provider/routing projections can refresh safely before any route mutation is considered?',
    safeActionIds: [...SAFE_ROUTING_ACTION_IDS],
    governedMutation: {
      label: 'Provider reroute',
      status: 'not exposed',
      governanceGate: 'route mutation requires a separate approval contract',
    },
    evidencePaths: [
      'core/state/charon_router.json',
      'core/state/provider_intelligence.json',
      'core/state/chronos_runtime.json',
      'core/state/provider_token_usage.json',
    ],
  }
}

export default function RoutingActionContractPanel({
  actionDescriptors,
  capabilityStatuses,
  busyActionId,
  message,
  onRunAction,
}: RoutingActionContractPanelProps) {
  const contract = buildRoutingActionContract()

  return (
    <section className="planning-action-contract routing-action-contract" aria-label="Routing action contract">
      <div className="module-subtitle"><Network size={14} /> {contract.title}</div>
      <article className="document-list__item">
        <div className="document-list__title-row">
          <strong>{contract.sourceZoneId}</strong>
          <span>{contract.safeActionIds.length} safe / mutation gated</span>
        </div>
        <p>{contract.operatorQuestion}</p>
        <dl className="operating-surface-capability-contract">
          <div><dt>Evidence</dt><dd>{contract.evidencePaths.join(', ')}</dd></div>
          <div><dt>Safe lane</dt><dd>provider metadata and runtime projection refresh only</dd></div>
          <div><dt>Route lane</dt><dd>{contract.governedMutation.governanceGate}</dd></div>
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
          <div className="planning-action-contract__command">
            <ShieldCheck size={14} />
            <code>
              {contract.governedMutation.label}: {contract.governedMutation.status} / {contract.governedMutation.governanceGate}
            </code>
          </div>
        </div>
      </article>
      {message ? <p className="planning-action-contract__message">{message}</p> : null}
    </section>
  )
}
