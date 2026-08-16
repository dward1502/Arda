import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import GovernanceGuardhouseWorkstation, {
  deriveGuardhouseSourceStates,
} from './GovernanceGuardhouseWorkstation'
import type { ReviewGateItem } from './ReviewGateWorkstation'

const records: ReviewGateItem[] = [
  {
    id: 'arandur-1',
    kind: 'queue_write',
    title: 'Promote bounded maintenance task',
    source: 'Arandur',
    status: 'pending_review',
    decisionClass: 'queue_write',
    evidence: 'data/arandur/mission_queue_write_requests.jsonl#arandur-1',
    summary: 'Append one reviewed task to the canonical queue.',
    checklist: ['scope is bounded'],
    createdAtUtc: '2026-08-15T10:00:00Z',
  },
  {
    id: 'athena-1',
    kind: 'athena_policy_readiness',
    title: 'Review doctrine evidence',
    source: 'ATHENA',
    status: 'reference_only',
    decisionClass: 'policy_readiness',
    evidence: 'data/athena/policy_readiness.jsonl#athena-1',
    summary: 'Evidence is reference-only and cannot authorize mutation.',
    checklist: ['retain reference-only scope'],
    createdAtUtc: '2026-08-14T08:00:00Z',
  },
]

const provenance = [
  {
    domainId: 'warden-edge',
    label: 'Warden edge contract',
    sourcePaths: ['core/state/warden_edge_contract.json'],
    generatedAtUtc: '2026-08-15T00:00:00Z',
    observedAtUtc: null,
    state: 'stale' as const,
    sourceKind: 'snapshot' as const,
  },
  {
    domainId: 'review-gates',
    label: 'Review gates',
    sourcePaths: ['data/arandur/mission_queue_write_requests.jsonl'],
    generatedAtUtc: '2026-08-15T00:00:00Z',
    observedAtUtc: null,
    state: 'fresh' as const,
    sourceKind: 'snapshot' as const,
  },
]

const baseProps = {
  governance: { ready: false, weights: [], thresholds: [] },
  governanceSignals: [{ label: 'Autonomy', value: '0.85' }],
  autonomyReadiness: { posture: 'read_only_closed_mutation_locked', checkpoint: [], evidence: [], nextUnlocks: [] },
  approvals: [],
  sourceProvenance: provenance,
  sourceCoverage: { status: 'partial' as const, label: 'source map partial', missingCount: 2 },
  busy: false,
  onDefer: vi.fn(),
  onApprove: vi.fn(),
  onReject: vi.fn(),
}

describe('GovernanceGuardhouseWorkstation', () => {
  it('renders a posture rail and honest Warden source states', () => {
    render(<GovernanceGuardhouseWorkstation {...baseProps} items={records} />)

    expect(screen.getByLabelText('Governance posture rail')).toHaveTextContent('read_only_closed_mutation_locked')
    expect(screen.getByLabelText('Guardhouse source states')).toHaveTextContent('Edge contract')
    expect(screen.getByLabelText('Guardhouse source states')).toHaveTextContent('stale snapshot')
    expect(screen.getByLabelText('Guardhouse source states')).toHaveTextContent('unavailable')
  })

  it('selects one record from the left index and shows only its detail on the right', () => {
    render(<GovernanceGuardhouseWorkstation {...baseProps} items={records} />)

    fireEvent.click(screen.getByRole('button', { name: /Review doctrine evidence/i }))

    expect(screen.getByLabelText('Selected governance record')).toHaveTextContent('Review doctrine evidence')
    expect(screen.getByLabelText('Selected governance record')).toHaveTextContent('reference-only')
    expect(screen.getByLabelText('Selected governance record')).not.toHaveTextContent('Append one reviewed task')
  })

  it('places evidence and authority before contextual decision controls', () => {
    render(<GovernanceGuardhouseWorkstation {...baseProps} items={records} />)

    const detail = screen.getByLabelText('Selected governance record')
    expect(detail).toHaveTextContent('Evidence')
    expect(detail).toHaveTextContent('Authority')
    expect(detail).toHaveTextContent('queue_write')
    expect(detail).toContainElement(screen.getByRole('button', { name: /^Defer$/i }))
    expect(detail).toContainElement(screen.getByRole('button', { name: /^Approve$/i }))
    expect(detail).toContainElement(screen.getByRole('button', { name: /^Reject$/i }))
  })

  it('distinguishes an available empty queue from an unavailable source', () => {
    const { rerender } = render(<GovernanceGuardhouseWorkstation {...baseProps} items={[]} />)
    expect(screen.getByText('No pending governance records')).toBeInTheDocument()

    rerender(<GovernanceGuardhouseWorkstation {...baseProps} items={[]} sourceProvenance={[]} />)
    expect(screen.getByText('Governance record source unavailable')).toBeInTheDocument()
  })

  it('classifies absent Warden contracts without implying they are live', () => {
    expect(deriveGuardhouseSourceStates(provenance)).toEqual([
      expect.objectContaining({ id: 'guardhouse', state: 'unavailable' }),
      expect.objectContaining({ id: 'edge-contract', state: 'stale snapshot' }),
      expect.objectContaining({ id: 'nightly-doctrine', state: 'unavailable' }),
      expect.objectContaining({ id: 'policy-authority', state: 'unavailable' }),
    ])
  })

  it('defers without invoking an append-only decision callback', () => {
    render(<GovernanceGuardhouseWorkstation {...baseProps} items={records} />)

    fireEvent.click(screen.getByRole('button', { name: /^Defer$/i }))

    expect(baseProps.onDefer).toHaveBeenCalledWith(records[0])
    expect(baseProps.onApprove).not.toHaveBeenCalled()
    expect(baseProps.onReject).not.toHaveBeenCalled()
  })

  it('preserves governed active-task authority as selected-record actions', () => {
    const onCancelTask = vi.fn()
    render(<GovernanceGuardhouseWorkstation
      {...baseProps}
      items={[]}
      activeTasks={[{
        id: 'task-7',
        title: 'Guarded execution',
        owner: 'arandur',
        status: 'running',
        priority: 'high',
        executionReceiptDigest: 'receipt-7',
      }]}
      onCancelTask={onCancelTask}
    />)

    fireEvent.click(screen.getByRole('button', { name: 'Cancel run' }))
    expect(onCancelTask).toHaveBeenCalledWith(expect.objectContaining({ id: 'task-7' }))
    expect(screen.getByLabelText('Selected governance record')).toHaveTextContent('receipt-7')
  })
})
