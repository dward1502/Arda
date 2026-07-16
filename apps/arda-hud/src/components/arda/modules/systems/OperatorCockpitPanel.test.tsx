import { render, screen } from '@testing-library/react'
import OperatorCockpitPanel from './OperatorCockpitPanel'

describe('OperatorCockpitPanel', () => {
  it('shows control-loop truth and explicit ledger gaps', () => {
    render(
      <OperatorCockpitPanel
        surface={{
          queue: {
            openTotal: 2,
            statusSplit: { ready: 1, pending: 1, inProgress: 0, blocked: 0 },
            items: [
              { id: 'tsk_1', title: 'Wire cockpit truth', owner: 'arda', status: 'queued', priority: 'high' },
            ],
          },
          humanGates: {
            blockedTotal: 1,
            items: [
              { id: 'gate_1', title: 'Approve gateway dispatch', status: 'pending_review', decisionClass: 'hermes_gateway' },
            ],
          },
          warden: {
            effectiveAttention: 0,
            rawAttention: 26,
            repeatedNoise: 26,
            activeRepairFiles: 0,
            resolvedRepairFiles: 12,
          },
          chronos: {
            runnerStatus: 'scheduled_audit_tasks_ready',
            readyTaskCount: 5,
            scheduledTaskCount: 5,
            dueTasks: [
              { id: 'chronos_1', name: 'Warden pressure audit', cadence: 'hourly', owner: 'chronos_warden' },
            ],
          },
          hermes: {
            gatewayReceiptCount: 1,
            dispatchReceiptCount: 1,
            latestReceipts: [
              { id: 'receipt_1', status: 'recorded', task: 'HG-P1', source: 'gateway' },
            ],
          },
          athena: {
            policyReady: 3,
            referenceOnly: 7,
            implementationReady: 0,
            latest: [
              { sourceId: 'src_1', readiness: 'reference_only', confidence: '0.70' },
            ],
          },
          charon: {
            providerCount: 12,
            availableProviderCount: 10,
            blockedProviderCount: 0,
            cooldownCount: 1,
            budgetPressureCount: 1,
            toolContextFloor: 64000,
            warnings: [
              { providerId: 'mistral', state: 'cooldown', level: 'critical', detail: 'rate limited' },
            ],
          },
          autonomyGate: {
            decision: 'operator_review',
            cleanupPacketCount: 1,
            externalSourceBlockedCount: 2,
            reasons: ['pending HADES cleanup review'],
          },
          storageHygiene: {
            status: 'warn',
            cleanupCandidateCount: 12,
            deletedBytes: 4096,
            warnings: [
              { label: 'latest_apply', value: '4096 bytes', detail: '3 deleted' },
            ],
          },
          ledgerGaps: [
            { label: 'Chronos audit receipts', path: 'data/chronos/audit_receipts.jsonl', status: 'missing', detail: 'ledger file is missing or unreadable' },
          ],
        }}
      />,
    )

    expect(screen.getByText('Control Loop Truth')).toBeInTheDocument()
    expect(screen.getByText('Wire cockpit truth | arda | queued | high')).toBeInTheDocument()
    expect(screen.getByText('Approve gateway dispatch | hermes_gateway | pending_review')).toBeInTheDocument()
    expect(screen.getByText('Warden pressure audit | chronos_warden | hourly')).toBeInTheDocument()
    expect(screen.getByText('gateway | recorded | HG-P1')).toBeInTheDocument()
    expect(screen.getByText('mistral | cooldown | critical | rate limited')).toBeInTheDocument()
    expect(screen.getByText('HADES packets 1 | external sources 2')).toBeInTheDocument()
    expect(screen.getByText('pending HADES cleanup review')).toBeInTheDocument()
    expect(screen.getByText('latest_apply | 4096 bytes | 3 deleted')).toBeInTheDocument()
    expect(screen.getByText('src_1 | reference_only | confidence 0.70')).toBeInTheDocument()
    expect(screen.getByText(/ledger file is missing or unreadable/)).toBeInTheDocument()
  })
})
