// sigil: REPAIR
import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import ArandurApprovalWorkstation, {
  type ArandurQueueWriteRequest,
  type HumanAugmentationApproval,
} from './ArandurApprovalWorkstation'
import type { StatefulPersona } from '../../../lib/statefulPersona'

const writeRequest: ArandurQueueWriteRequest = {
  id: 'queue-write-arandur-social-scout',
  missionCandidateId: 'arandur-social-scout',
  queueProposalId: 'queue-proposal-arandur-social-scout',
  title: 'Launch Arandur social scout automation',
  scope: 'arandur_automation',
  justification: 'Structured request for public-internet scouting with bounded queue mutation.',
  createdAtUtc: '2026-05-18T04:00:00Z',
  canonicalQueueSha1: 'before-sha',
  proposalSha1: 'proposal-sha',
  reviewRequired: true,
  reviewChecklist: [
    'Confirm mission scope is bounded',
    'Verify no canonical queue write has happened yet',
  ],
  requiresFutureHumanApproval: true,
  requiresSeparateFutureCanonicalQueueWrite: true,
  mutationPolicy: {
    canonical_queue: 'read_only',
    output_ledger: 'append_only',
  },
  writePending: true,
  executionStatus: 'write_pending',
  canonicalQueueTaskId: null,
}

const approval: HumanAugmentationApproval = {
  id: 'approval-arandur-social-scout',
  decisionClass: 'queue_write',
  approvers: 'aurelius, bacon',
  status: 'pending',
  note: 'Awaiting operator approval for Arandur queue write.',
  commandSignature: 'queue-write-arandur-social-scout',
}

const projectedPersona: StatefulPersona = {
  actor: 'arandur',
  status: 'ready',
  sourceRecordId: 'persona:arandur:projection',
  traits: [
    { traitId: 'direct', label: 'Direct', evidenceCount: 4, confidence: 0.4, stale: false },
    { traitId: 'curious', label: 'Curious', evidenceCount: 3, confidence: 0.3, stale: true },
  ],
  moodSummary: {
    asOf: '2026-08-03T12:00:00Z',
    weightedValence: 0.42,
    sampleCount: 6,
    windowHours: 336,
  },
  message: 'Persona projection loaded from Vairë.',
}

describe('ArandurApprovalWorkstation', () => {
  it('renders queue write request details and safety gates for operator review', () => {
    render(
      <ArandurApprovalWorkstation
        approvals={[approval]}
        queueWriteRequests={[writeRequest]}
        busy={false}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByRole('heading', { name: /Arandur Approval Workstation/i })).toBeInTheDocument()
    expect(screen.getByText('Launch Arandur social scout automation')).toBeInTheDocument()
    expect(screen.getByText('queue-write-arandur-social-scout')).toBeInTheDocument()
    expect(screen.getByText(/Structured request for public-internet scouting/i)).toBeInTheDocument()
    expect(screen.getByText('Confirm mission scope is bounded')).toBeInTheDocument()
    expect(screen.getByText('canonical_queue: read_only')).toBeInTheDocument()
    expect(screen.getByText('requires separate future canonical queue write')).toBeInTheDocument()
    expect(screen.getByText('write pending')).toBeInTheDocument()
    expect(screen.getByText(/Awaiting operator approval/i)).toBeInTheDocument()
  })

  it('renders approved tasks from the active universal queue', () => {
    render(
      <ArandurApprovalWorkstation
        approvals={[]}
        queueWriteRequests={[]}
        activeTasks={[{
          id: 'tsk-active-1',
          title: 'Validate active queue dispatch',
          owner: 'prometheus',
          status: 'pending',
          priority: 'high',
        }]}
        busy={false}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByText('Active Universal Queue')).toBeInTheDocument()
    expect(screen.getByText('Validate active queue dispatch')).toBeInTheDocument()
    expect(screen.getByText('prometheus / high')).toBeInTheDocument()
  })

  it('renders Workbench lineage and routes governed cancellation and retry', () => {
    const onCancelTask = vi.fn()
    const onRetryTask = vi.fn()
    render(
      <ArandurApprovalWorkstation
        approvals={[]}
        queueWriteRequests={[]}
        activeTasks={[
          {
            id: 'running-task', title: 'Running task', owner: 'Arandur', status: 'in_progress', priority: 'high',
            workbenchRunId: 'queue-running-task', leaseExpiresAtUtc: '2026-08-13T12:00:00Z', executionReceiptDigest: 'sha256:1234567890abcdef',
          },
          {
            id: 'failed-task', title: 'Failed task', owner: 'Arandur', status: 'failed', priority: 'low', result: 'failed', detail: 'provider failed',
          },
        ]}
        busy={false}
        onApprove={vi.fn()}
        onReject={vi.fn()}
        onCancelTask={onCancelTask}
        onRetryTask={onRetryTask}
      />,
    )
    expect(screen.getByText('Run queue-running-task')).toBeInTheDocument()
    expect(screen.getByText('Receipt sha256:12345')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Cancel run' }))
    fireEvent.click(screen.getByRole('button', { name: 'Retry governed task' }))
    expect(onCancelTask).toHaveBeenCalledWith(expect.objectContaining({ id: 'running-task' }))
    expect(onRetryTask).toHaveBeenCalledWith(expect.objectContaining({ id: 'failed-task' }))
  })

  it('emits approve and reject actions with the selected queue write request', () => {
    const onApprove = vi.fn()
    const onReject = vi.fn()

    render(
      <ArandurApprovalWorkstation
        approvals={[approval]}
        queueWriteRequests={[writeRequest]}
        busy={false}
        onApprove={onApprove}
        onReject={onReject}
      />,
    )

    fireEvent.click(screen.getByRole('button', { name: /Approve queue write/i }))
    expect(onApprove).toHaveBeenCalledWith(writeRequest)

    fireEvent.click(screen.getByRole('button', { name: /Reject queue write/i }))
    expect(onReject).toHaveBeenCalledWith(writeRequest)
  })

  it('keeps approval controls disabled when no queue write request is available', () => {
    render(
      <ArandurApprovalWorkstation
        approvals={[]}
        queueWriteRequests={[]}
        busy={false}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByText(/No Arandur queue write requests detected/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Approve queue write/i })).toBeDisabled()
    expect(screen.getByRole('button', { name: /Reject queue write/i })).toBeDisabled()
  })

  it('renders memory-backed traits, stale state, and mood without static persona text', () => {
    render(
      <ArandurApprovalWorkstation
        approvals={[]}
        queueWriteRequests={[]}
        busy={false}
        persona={projectedPersona}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByText('Personality')).toBeInTheDocument()
    expect(screen.getByText(/Direct.*40%.*4 evidence/i)).toBeInTheDocument()
    expect(screen.getByText(/Curious.*stale/i)).toBeInTheDocument()
    expect(screen.getByText(/Mood.*\+0.42.*6 samples/i)).toBeInTheDocument()
  })

  it('shows a neutral unavailable state instead of invented traits', () => {
    render(
      <ArandurApprovalWorkstation
        approvals={[]}
        queueWriteRequests={[]}
        busy={false}
        persona={{ ...projectedPersona, status: 'unavailable', traits: [], moodSummary: null, message: 'Persona projection unavailable.' }}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByText('Persona projection unavailable.')).toBeInTheDocument()
    expect(screen.queryByText(/Direct.*40%/i)).not.toBeInTheDocument()
  })
})
