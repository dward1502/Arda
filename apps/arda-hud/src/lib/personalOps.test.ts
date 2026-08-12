import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }))

import { createPersonalOpsClient } from './personalOps'

describe('Personal Operations Rust authority client', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    ;(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {}
  })

  it('loads one versioned aggregate projection through Tauri', async () => {
    invokeMock.mockResolvedValue({ schemaVersion: 'arda.hud.personal-ops-projection.v1' })
    await createPersonalOpsClient().loadSnapshot()
    expect(invokeMock).toHaveBeenCalledWith('get_personal_ops_projection', undefined)
  })

  it('submits bounded intent without browser authority fields', async () => {
    invokeMock.mockResolvedValue({ event_id: 'event-1', capture_id: 'capture-1' })
    await createPersonalOpsClient().createCapture('Buy tea')
    expect(invokeMock).toHaveBeenCalledWith('create_personal_capture', {
      intent: { text: 'Buy tea' },
    })
    expect(JSON.stringify(invokeMock.mock.calls)).not.toMatch(/operatorId|idempotency|occurredAt/)
  })

  it('keeps identity, evidence and receipts out of classification intent', async () => {
    invokeMock.mockResolvedValue({ event_id: 'event-2' })
    await createPersonalOpsClient().confirmClassification('item-1', 'task')
    expect(invokeMock).toHaveBeenCalledWith('confirm_personal_classification', {
      intent: { itemId: 'item-1', kind: 'task' },
    })
    expect(JSON.stringify(invokeMock.mock.calls)).not.toMatch(/operatorId|evidenceClass|rationale|eventId/)
  })

  it('requires the Rust command to validate the fixed destructive confirmation', async () => {
    invokeMock.mockResolvedValue({ receipt_id: 'receipt-1', deleted_events: 1 })
    await createPersonalOpsClient().deletePersonalData()
    expect(invokeMock).toHaveBeenCalledWith('delete_personal_data', {
      intent: { confirmation: 'delete-personal-data' },
    })
  })
})
