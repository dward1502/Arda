// sigil: REPAIR
import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import ReviewGateWorkstation, { buildReviewGateDecisionRecordPreview, type ReviewGateItem } from './ReviewGateWorkstation'
import type { HumanAugmentationApproval } from './ArandurApprovalWorkstation'

const reviewItem: ReviewGateItem = {
  id: 'phase6g-approval-demo',
  kind: 'mission_approval',
  title: 'Review mission candidate: social scout',
  source: 'public internet opportunity scouting',
  status: 'pending_review',
  decisionClass: 'arandur_mission_approval',
  evidence: 'canonical-sha,mission-sha,review-sha',
  summary: 'Should a future explicit operator action promote this reviewed mission?',
  checklist: [
    'confirm the Phase 6F review packet is still valid',
    'confirm this request does not mutate the canonical task queue',
  ],
  createdAtUtc: '2026-05-18T08:42:45Z',
}

const approval: HumanAugmentationApproval = {
  id: 'approval-phase6g-approval-demo',
  decisionClass: 'arandur_mission_approval',
  approvers: 'aurelius, bacon',
  status: 'approved',
  note: 'Approved mission packet for bounded planning.',
  commandSignature: 'phase6g-approval-demo',
}

describe('ReviewGateWorkstation', () => {
  it('renders unified review packets with checklist and approval state', () => {
    render(
      <ReviewGateWorkstation
        items={[reviewItem]}
        approvals={[approval]}
        busy={false}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByRole('heading', { name: /Review Gate/i })).toBeInTheDocument()
    expect(screen.getAllByText('Review mission candidate: social scout')).toHaveLength(2)
    expect(screen.getAllByText(/public internet opportunity scouting/i)).toHaveLength(2)
    expect(screen.getByText('confirm the Phase 6F review packet is still valid')).toBeInTheDocument()
    expect(screen.getByText(/Approved mission packet/i)).toBeInTheDocument()
  })

  it('renders the selected packet as concise plain-language decision copy instead of raw JSON', () => {
    render(
      <ReviewGateWorkstation
        items={[reviewItem]}
        approvals={[]}
        busy={false}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByText('Action requested')).toBeInTheDocument()
    expect(screen.getAllByText(/Review mission candidate: social scout/).length).toBeGreaterThan(0)
    expect(screen.getByText(/Approve only records an operator decision/i)).toBeInTheDocument()
    expect(screen.queryByText(/\{"id"/)).not.toBeInTheDocument()
  })

  it('shows the inspect-decide-record workflow and decision record preview', () => {
    render(
      <ReviewGateWorkstation
        items={[reviewItem]}
        approvals={[]}
        busy={false}
        decisionApprovers="operator"
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByLabelText('Review gate workflow')).toHaveTextContent('Inspect')
    expect(screen.getByLabelText('Review gate workflow')).toHaveTextContent('Decide')
    expect(screen.getByLabelText('Review gate workflow')).toHaveTextContent('Record')
    expect(screen.getByText('Decision record preview')).toBeInTheDocument()
    expect(screen.getByText('decision_class')).toBeInTheDocument()
    expect(screen.getAllByText('arandur_mission_approval').length).toBeGreaterThanOrEqual(1)
    expect(screen.getByText('command_signature')).toBeInTheDocument()
    expect(screen.getAllByText('phase6g-approval-demo').length).toBeGreaterThanOrEqual(1)
    expect(screen.getByText('operator')).toBeInTheDocument()
    expect(screen.getByText('arda-hud,canonical-sha,mission-sha,review-sha')).toBeInTheDocument()
  })

  it('builds the same decision preview payload used by the workstation', () => {
    expect(buildReviewGateDecisionRecordPreview(reviewItem, 'operator')).toEqual({
      decisionClass: 'arandur_mission_approval',
      commandSignature: 'phase6g-approval-demo',
      approvers: 'operator',
      evidence: 'arda-hud,canonical-sha,mission-sha,review-sha',
      approvalNote: 'Approved arandur_mission_approval phase6g-approval-demo: Review mission candidate: social scout',
      rejectionNote: 'Rejected arandur_mission_approval phase6g-approval-demo: Review mission candidate: social scout',
    })
  })

  it('emits approve and reject decisions with the selected packet', () => {
    const onApprove = vi.fn()
    const onReject = vi.fn()

    render(
      <ReviewGateWorkstation
        items={[reviewItem]}
        approvals={[]}
        busy={false}
        onApprove={onApprove}
        onReject={onReject}
      />,
    )

    fireEvent.click(screen.getByRole('button', { name: /^Approve$/i }))
    expect(onApprove).toHaveBeenCalledWith(reviewItem)

    fireEvent.click(screen.getByRole('button', { name: /Reject/i }))
    expect(onReject).toHaveBeenCalledWith(reviewItem)
  })

  it('shows provenance freshness for the selected review packet evidence', () => {
    render(
      <ReviewGateWorkstation
        items={[{ ...reviewItem, source: 'review-gates', evidence: 'data/arandur/recommendations.jsonl' }]}
        approvals={[]}
        busy={false}
        sourceProvenance={[
          {
            domainId: 'review-gates',
            label: 'Review Gates',
            sourcePaths: ['data/arandur/recommendations.jsonl'],
            generatedAtUtc: '2026-06-01T00:00:00.000Z',
            observedAtUtc: null,
            state: 'fresh',
            sourceKind: 'snapshot',
          },
        ]}
        onApprove={vi.fn()}
        onReject={vi.fn()}
      />,
    )

    expect(screen.getByText('Source Freshness')).toBeInTheDocument()
    expect(screen.getByText('Review Gates')).toBeInTheDocument()
    expect(screen.getByText('Fresh')).toBeInTheDocument()
  })
})
