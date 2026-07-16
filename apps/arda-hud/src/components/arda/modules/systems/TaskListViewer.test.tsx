import { fireEvent, render, screen } from '@testing-library/react'
import TaskListViewer, { deriveTaskSourcePath } from './TaskListViewer'
import type { OperatorCockpitSurface } from './OperatorCockpitPanel'

const surface: OperatorCockpitSurface = {
  queue: {
    openTotal: 2,
    statusSplit: { ready: 0, pending: 2, inProgress: 0, blocked: 0 },
    items: [
      {
        id: 'tsk_20260605_arda_task_viewer',
        title: 'ARDA-HUD task-list viewer component',
        owner: 'prometheus',
        status: 'queued',
        priority: 'high',
      },
      {
        id: 'tsk_20260605_provider-matrix-hardening-contract_impl',
        title: 'Implement contract: provider-matrix-hardening-contract',
        owner: 'prometheus',
        status: 'queued',
        priority: 'high',
      },
    ],
  },
  humanGates: { blockedTotal: 0, items: [] },
  warden: { effectiveAttention: 0, rawAttention: 0, repeatedNoise: 0, activeRepairFiles: 0, resolvedRepairFiles: 0 },
  chronos: { runnerStatus: 'ready', readyTaskCount: 0, scheduledTaskCount: 0, dueTasks: [] },
  hermes: { gatewayReceiptCount: 0, dispatchReceiptCount: 0, latestReceipts: [] },
  athena: { policyReady: 0, referenceOnly: 0, implementationReady: 0, latest: [] },
  charon: {
    providerCount: 0,
    availableProviderCount: 0,
    blockedProviderCount: 0,
    cooldownCount: 0,
    budgetPressureCount: 0,
    toolContextFloor: 0,
    warnings: [],
  },
  autonomyGate: { decision: 'ready', cleanupPacketCount: 0, externalSourceBlockedCount: 0, reasons: [] },
  storageHygiene: { status: 'missing', cleanupCandidateCount: 0, deletedBytes: 0, warnings: [] },
  ledgerGaps: [],
}

describe('TaskListViewer', () => {
  it('shows open tasks and derives task source paths', () => {
    const approved: string[] = []
    render(<TaskListViewer surface={surface} onApprove={(taskId) => approved.push(taskId)} />)

    expect(screen.getByText('Open Work Viewer')).toBeInTheDocument()
    expect(screen.getAllByText('ARDA-HUD task-list viewer component')).toHaveLength(2)
    expect(screen.getByText('source: docs/plans/arda_task_viewer_plan.md')).toBeInTheDocument()

    fireEvent.click(screen.getByText('Implement contract: provider-matrix-hardening-contract'))
    expect(screen.getByText('source: docs/contracts/provider-matrix-hardening-contract.md')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: /Approve/ }))
    expect(approved).toEqual(['tsk_20260605_provider-matrix-hardening-contract_impl'])
  })

  it('derives recovered plan source paths', () => {
    expect(deriveTaskSourcePath({
      id: 'tsk_20260605_plan_impl',
      title: 'Implement plan: 2026-06-05-queue-backlog-decomposition-and-hygiene-plan',
      owner: 'prometheus',
      status: 'queued',
      priority: 'high',
    })).toBe('docs/plans/2026-06-05-queue-backlog-decomposition-and-hygiene-plan.md')
  })
})
