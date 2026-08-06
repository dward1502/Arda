import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import PersonalOperationsModule from './PersonalOperationsModule'
import type { PersonalOpsClient, PersonalOpsSnapshot } from '../../../lib/personalOps'

const snapshot: PersonalOpsSnapshot = {
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
    acknowledgeReminder: vi.fn(async () => ({ event_id: 'event-2' })),
    ...overrides,
  }
}

describe('PersonalOperationsModule', () => {
  it('renders the backend-backed first screen without requiring categorization', async () => {
    render(<PersonalOperationsModule client={client()} operatorId="operator" />)

    expect(await screen.findByRole('heading', { name: 'Personal Operations' })).toBeInTheDocument()
    expect(screen.getByLabelText('Rapid capture')).toBeInTheDocument()
    expect(screen.queryByLabelText(/category/i)).not.toBeInTheDocument()
    expect(screen.getAllByText('Prepare launch checklist')).toHaveLength(2)
    expect(screen.getByText('1 reminder awaiting acknowledgement')).toBeInTheDocument()
    expect(screen.getByText('Quiet mode unavailable')).toBeInTheDocument()
    expect(screen.getByText(/placeholder/i)).toBeInTheDocument()
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
    ack.focus()
    fireEvent.keyDown(ack, { key: 'Enter' })
    await waitFor(() => expect(ops.acknowledgeReminder).toHaveBeenCalledWith('reminder-1'))
    expect(container.querySelector('.personal-ops--reduced-motion')).toBeTruthy()
    expect(container.querySelector('.personal-ops--high-contrast')).toBeTruthy()
  })

  it('announces loading, error, and empty states accessibly', async () => {
    const emptyClient = client({
      loadSnapshot: vi.fn(async () => ({
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
})
