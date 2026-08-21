import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import PersonalOperationsModule from './PersonalOperationsModule'
import type { PersonalOpsClient, PersonalOpsSnapshot } from '../../../lib/personalOps'

const snapshot: PersonalOpsSnapshot = {
  nextAction: {
    schema_version: 'arda.next-action.v1',
    generated_at: '2026-08-04T09:00:00Z',
    status: 'ready',
    reason: 'Highest-priority current operator-authored commitment.',
    excluded: { stale: 0, terminal: 0, future_gated: 2, inferred_without_review: 0 },
    selected: {
      id: 'operator-core-review',
      title: 'Review core Arda against the operator vision',
      source_kind: 'queue',
      source_ref: 'core/projects/tasks/queue.jsonl#operator-core-review',
      reason: 'Highest-priority current operator-authored commitment.',
      freshness: 'fresh',
      authority_state: 'review_required',
      next_operator_action: 'Review this objective and explicitly start, revise, or defer it.',
      priority: 90,
      operator_authored: true,
      terminal: false,
      future_gated: false,
      inferred_without_review: false,
    },
  },
  inbox: {
    schema_version: 'arda.harness.personal-ops.v1',
    inbox: [
      {
        capture_id: 'capture-1',
        operator_id: 'operator',
        content: 'Unsorted idea from the morning',
        audio_reference: null,
        occurred_at: '2026-08-04T08:10:00Z',
      },
    ],
  },
  resume: {
    schema_version: 'arda.harness.personal-ops.v1',
    resume: {
      summary: '1 capture(s) in the inbox awaiting classification.',
      active_count: 1,
      inbox_count: 1,
      today_count: 1,
      waiting_count: 1,
      generated_at: '2026-08-04T09:00:00Z',
    },
  },
  todayBrief: {
    schema_version: 'arda.harness.personal-ops.v1',
    brief: {
      generated_at: '2026-08-04T09:00:00Z',
      today: [
        {
          item_id: 'item-1',
          kind: 'task',
          operator_id: 'operator',
          content: 'Prepare launch checklist',
          evidence_class: 'operator_authored',
          confidence: null,
          classification_reason: 'operator_input',
          scheduled_at: null,
          due_at: '2026-08-04T18:00:00Z',
          completed_at: null,
          reminder_id: 'reminder-1',
          reminder_state: {
            delivery_state: 'attempted',
            attempt_count: 1,
            last_acknowledged_at: null,
            policy: {
              interruption: 'quiet_window_aware',
              quiet_window: null,
              max_attempts: 3,
              minimum_interval_minutes: 15,
              acknowledgement_required: true,
            },
            non_clinical_disclosure: 'Wellness assistance only; this record is not clinical measurement or medical advice.',
          },
          reminder_attempts: 1,
          reminder_acknowledged_at: null,
          current_state: 'active',
        },
      ],
      waiting: [],
      reminders_awaiting_ack: 1,
      quiet_mode: false,
      uncertainty_disclosure: 'Brief reconstructed from local event log; items may change as captures are reclassified.',
    },
  },
}

function client(overrides: Partial<PersonalOpsClient> = {}): PersonalOpsClient {
  return {
    loadSnapshot: vi.fn(async () => snapshot),
    createCapture: vi.fn(async () => ({ event_id: 'event-1', capture_id: 'capture-2' })),
    confirmClassification: vi.fn(async () => ({ event_id: 'event-classify' })),
    scheduleItem: vi.fn(async () => ({ event_id: 'event-schedule' })),
    completeItem: vi.fn(async () => ({ event_id: 'event-complete' })),
    acknowledgeReminder: vi.fn(async () => ({ event_id: 'event-2' })),
    respondToReminder: vi.fn(async () => ({ event_id: 'event-reminder-response' })),
    exportPersonalData: vi.fn(async () => ({
      schema_version: 'arda.personal-data-export.v1' as const,
      generated_at: '2026-08-06T14:00:00Z',
      operator_id: 'operator',
      events: [{}],
    })),
    deletePersonalData: vi.fn(async () => ({
      receipt_id: 'delete-receipt',
      deleted_events: 1,
      system_receipts_modified: false as const,
    })),
    ...overrides,
  }
}

describe('PersonalOperationsModule', () => {
  it('renders the backend-backed first screen without requiring categorization', async () => {
    render(<PersonalOperationsModule client={client()} operatorId="operator" />)

    expect(await screen.findByRole('heading', { name: 'Personal Operations' })).toBeInTheDocument()
    expect(screen.getByLabelText('Rapid capture')).toBeInTheDocument()
    expect(screen.queryByLabelText(/category/i)).not.toBeInTheDocument()
    expect(screen.getByText('Review core Arda against the operator vision')).toBeInTheDocument()
    expect(screen.getByText(/queue · fresh · review required/i)).toBeInTheDocument()
    expect(screen.getAllByText('Prepare launch checklist')).toHaveLength(1)
    expect(screen.getByText('1 reminder awaiting acknowledgement')).toBeInTheDocument()
    expect(screen.getByText('Quiet mode unavailable')).toBeInTheDocument()
    expect(screen.getByText(/Calendar sync: not configured/)).toBeInTheDocument()
  })

  it('submits capture with Enter from the focused textarea and preserves newline with Shift+Enter', async () => {
    const ops = client()
    render(<PersonalOperationsModule client={ops} operatorId="operator" />)
    const capture = await screen.findByLabelText('Rapid capture')

    fireEvent.change(capture, { target: { value: 'Buy tea' } })
    fireEvent.keyDown(capture, { key: 'Enter', shiftKey: true })
    expect(ops.createCapture).not.toHaveBeenCalled()

    fireEvent.keyDown(capture, { key: 'Enter' })
    await waitFor(() => expect(ops.createCapture).toHaveBeenCalledWith('Buy tea'))
    expect(capture).toHaveValue('')
  })

  it('exposes screen-reader status, keyboard acknowledgement, reduced-motion and high-contrast posture', async () => {
    const ops = client()
    const { container } = render(<PersonalOperationsModule client={ops} operatorId="operator" />)

    expect(await screen.findByRole('status')).toHaveTextContent('Personal operations loaded')
    const ack = screen.getByRole('button', { name: 'Acknowledge reminder for Prepare launch checklist' })
    fireEvent.click(ack)
    await waitFor(() => expect(ops.acknowledgeReminder).toHaveBeenCalledWith('reminder-1'))
    expect(container.querySelector('.personal-ops--reduced-motion')).toBeTruthy()
    expect(container.querySelector('.personal-ops--high-contrast')).toBeTruthy()
  })

  it('announces loading, error, and empty states accessibly', async () => {
    const emptyClient = client({
      loadSnapshot: vi.fn(async () => ({
        nextAction: { schema_version: 'arda.next-action.v1', generated_at: '2026-08-04T09:00:00Z', status: 'empty', selected: null, reason: 'No current trustworthy action is available.', excluded: { stale: 0, terminal: 0, future_gated: 0, inferred_without_review: 0 } } as const,
        inbox: { schema_version: 'arda.harness.personal-ops.v1', inbox: [] },
        resume: { schema_version: 'arda.harness.personal-ops.v1', resume: { summary: 'Nothing in progress. Check your captures or scheduled items.', active_count: 0, inbox_count: 0, today_count: 0, waiting_count: 0, generated_at: '2026-08-04T09:00:00Z' } },
        todayBrief: { schema_version: 'arda.harness.personal-ops.v1', brief: { generated_at: '2026-08-04T09:00:00Z', today: [], waiting: [], reminders_awaiting_ack: 0, quiet_mode: false, uncertainty_disclosure: 'Brief reconstructed from local event log.' } },
      })),
    })
    render(<PersonalOperationsModule client={emptyClient} operatorId="operator" />)
    expect(screen.getByRole('status')).toHaveTextContent('Loading personal operations')
    expect(await screen.findByText('No timeline items for today.')).toBeInTheDocument()

    const failingClient = client({ loadSnapshot: vi.fn(async () => { throw new Error('network offline') }) })
    render(<PersonalOperationsModule client={failingClient} operatorId="operator" />)
    expect(await screen.findByRole('alert')).toHaveTextContent('network offline')
  })

  it('classifies inbox captures and exposes scheduling, completion, defer, and dismiss', async () => {
    const ops = client()
    render(<PersonalOperationsModule client={ops} operatorId="operator" />)
    await screen.findByRole('heading', { name: 'Personal Operations' })

    fireEvent.change(screen.getByLabelText('Classify as'), { target: { value: 'note' } })
    fireEvent.click(screen.getByRole('button', { name: 'Confirm classification' }))
    await waitFor(() => expect(ops.confirmClassification).toHaveBeenCalledWith('capture-1', 'note'))

    fireEvent.change(screen.getByLabelText('Schedule'), { target: { value: '2026-08-21T09:30' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save schedule' }))
    await waitFor(() => expect(ops.scheduleItem).toHaveBeenCalledWith('item-1', expect.stringContaining('2026-08-21T')))

    fireEvent.click(screen.getByRole('button', { name: 'Mark complete' }))
    await waitFor(() => expect(ops.completeItem).toHaveBeenCalledWith('item-1'))
    fireEvent.click(screen.getByRole('button', { name: 'Defer' }))
    await waitFor(() => expect(ops.respondToReminder).toHaveBeenCalledWith('reminder-1', 'deferred'))
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }))
    await waitFor(() => expect(ops.respondToReminder).toHaveBeenCalledWith('reminder-1', 'dismissed'))
  })

  it('exports personal data and requires an explicit second action before deletion', async () => {
    const ops = client()
    const download = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {})
    render(<PersonalOperationsModule client={ops} operatorId="operator" />)
    await screen.findByRole('heading', { name: 'Personal Operations' })

    fireEvent.click(screen.getByRole('button', { name: 'Export personal data' }))
    await waitFor(() => expect(ops.exportPersonalData).toHaveBeenCalledTimes(1))
    expect(download).toHaveBeenCalledTimes(1)
    expect(screen.getByRole('status').textContent).toContain('Personal data export ready with 1 event')

    fireEvent.click(screen.getByRole('button', { name: 'Delete personal data' }))
    expect(ops.deletePersonalData).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Confirm delete personal data' }))
    await waitFor(() => expect(ops.deletePersonalData).toHaveBeenCalledTimes(1))
    expect(screen.getByRole('status').textContent).toContain('Deleted 1 personal event; system receipts preserved')
    download.mockRestore()
  })
})
