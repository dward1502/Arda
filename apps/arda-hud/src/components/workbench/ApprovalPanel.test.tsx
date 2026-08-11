import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import ApprovalPanel from './ApprovalPanel'
import type { RunNode } from '../../lib/workbench'

const approval: RunNode = { id: 'approval-1', kind: 'approval', state: 'blocked', authority: 'human_approval', budget: { max_joules: 1, max_cost_usd: 0 }, retry: { max_attempts: 1 }, timeout_ms: 1000, idempotency_key: 'key', input_digest: null, output_digest: null, parent_receipts: [], checkpoint: { sequence: 0, recovery_token: null, checkpoint_digest: null } }

describe('ApprovalPanel', () => {
  it('lets a keyboard operator approve the selected gate', () => {
    const onApprove = vi.fn(); render(<ApprovalPanel approvals={[approval]} onApprove={onApprove} />)
    const button = screen.getByRole('button', { name: 'Approve' }); button.focus(); fireEvent.click(button)
    expect(document.activeElement).toBe(button); expect(onApprove).toHaveBeenCalledWith('approval-1')
  })

  it('exposes explicit rejection for revision', () => {
    const onReject = vi.fn(); render(<ApprovalPanel approvals={[approval]} onApprove={vi.fn()} onReject={onReject} />)
    fireEvent.click(screen.getByRole('button', { name: 'Reject' }))
    expect(onReject).toHaveBeenCalledWith('approval-1')
  })
})
