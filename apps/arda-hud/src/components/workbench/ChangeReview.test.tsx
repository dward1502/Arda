import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import ChangeReview from './ChangeReview'

describe('ChangeReview', () => {
  it('reports changed files, diffs, and test receipts', () => {
    render(<ChangeReview changes={[{ path: 'src/lib.rs', status: 'modified', additions: 2, deletions: 1, diff: '+safe boundary' }]} tests={[{ name: 'cargo test', status: 'passed', durationMs: 42 }]} providerReceipt={{ provider: 'nous', model: 'fixture-model', adapter: 'hermes-workbench', receipt_digest: 'sha256:provider', summary: 'Bounded mutation completed.' }} />)
    expect(screen.getByText('src/lib.rs')).toBeTruthy(); expect(screen.getByText(/passed \/ 42 ms/)).toBeTruthy()
    expect(screen.getByText('+safe boundary')).toBeTruthy()
    expect(screen.getByText('nous / fixture-model')).toBeTruthy(); expect(screen.getByText('Bounded mutation completed.')).toBeTruthy()
  })
  it('makes absent evidence explicit', () => {
    render(<ChangeReview changes={[]} tests={[]} />)
    expect(screen.getByText('No changes recorded.')).toBeTruthy(); expect(screen.getByText('No tests have run.')).toBeTruthy()
    expect(screen.getByText('No provider receipt recorded.')).toBeTruthy()
  })
  it('offers live provider execution for an unfinished execute node', () => {
    const execute = vi.fn()
    render(<ChangeReview changes={[]} tests={[]} selectedNode={{ id: 'execute', kind: 'execute', state: 'pending', authority: 'execute_with_approval', budget: { max_joules: 250, max_cost_usd: 2 }, retry: { max_attempts: 1 }, timeout_ms: 900_000, idempotency_key: 'run-execute', input_digest: 'objective:1', output_digest: null, parent_receipts: ['receipt:approval'], checkpoint: { sequence: 0, recovery_token: null, checkpoint_digest: null } }} onExecuteProvider={execute} />)
    fireEvent.click(screen.getByRole('button', { name: 'Execute approved node with live provider' }))
    expect(execute).toHaveBeenCalledOnce()
    expect(screen.queryByRole('button', { name: 'Record execute receipt' })).toBeNull()
  })
})
