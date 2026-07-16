// sigil: REPAIR
import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import ArandurApprovalWorkstation, {
  type ArandurQueueWriteRequest,
  type HumanAugmentationApproval,
} from './ArandurApprovalWorkstation'

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
})
