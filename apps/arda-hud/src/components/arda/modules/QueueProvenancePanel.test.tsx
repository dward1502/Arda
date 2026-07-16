import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import QueueProvenancePanel from './QueueProvenancePanel'

const records: ArdaSourceProvenance[] = [
  {
    domainId: 'planning_and_queue:core/projects/tasks/queue.jsonl',
    label: 'Planning And Queue / Task Queue',
    sourcePaths: ['core/projects/tasks/queue.jsonl'],
    generatedAtUtc: '2026-06-01T00:00:00.000Z',
    observedAtUtc: null,
    state: 'stale',
    sourceKind: 'manual',
  },
  {
    domainId: 'planning_and_queue:core/state/queue_summary.json',
    label: 'Planning And Queue / Queue Summary',
    sourcePaths: ['core/state/queue_summary.json'],
    generatedAtUtc: '2026-06-01T00:00:00.000Z',
    observedAtUtc: null,
    state: 'fresh',
    sourceKind: 'snapshot',
  },
  {
    domainId: 'world:core/state/world.json',
    label: 'World Runtime',
    sourcePaths: ['core/state/world.json'],
    generatedAtUtc: '2026-06-01T00:00:00.000Z',
    observedAtUtc: null,
    state: 'fresh',
    sourceKind: 'snapshot',
  },
]

const queueFederation = {
  sources: [
    {
      id: 'canonical_project_tasks',
      default_record_class: 'execution_attempt',
      lane_subclass: 'canonical_task',
      promotion_receipt_required: 'none_already_canonical',
    },
    {
      id: 'flywheel_packet_runtime',
      default_record_class: 'proposal',
      lane_subclass: 'flywheel_packet',
      promotion_receipt_required: 'flywheel_plan_packet_readiness_receipt',
    },
  ],
}

describe('QueueProvenancePanel', () => {
  it('renders queue freshness and approval-gated refresh guidance', () => {
    render(<QueueProvenancePanel records={records} queueFederation={queueFederation} />)

    expect(screen.getByRole('region', { name: 'Queue source freshness' })).toBeInTheDocument()
    expect(screen.getByLabelText('Queue stage contracts')).toBeInTheDocument()
    expect(screen.getByText('canonical_project_tasks')).toBeInTheDocument()
    expect(screen.getByText('flywheel_packet_runtime')).toBeInTheDocument()
    expect(screen.getByText('lane: flywheel_packet; receipt: flywheel_plan_packet_readiness_receipt')).toBeInTheDocument()
    expect(screen.getByText('Planning And Queue / Task Queue')).toBeInTheDocument()
    expect(screen.getByText('Planning And Queue / Queue Summary')).toBeInTheDocument()
    expect(screen.getAllByText('Approval required')).toHaveLength(2)
    expect(screen.getByText('Stale')).toBeInTheDocument()
    expect(screen.queryByText('World Runtime')).not.toBeInTheDocument()
  })

  it('renders nothing when no queue records exist', () => {
    const { container } = render(<QueueProvenancePanel records={[records[2]]} />)

    expect(container).toBeEmptyDOMElement()
  })
})
