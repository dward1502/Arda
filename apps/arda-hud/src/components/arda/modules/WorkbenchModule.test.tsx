import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { safeTauriInvoke } from '../../../lib/tauriGuard'
import WorkbenchModule from './WorkbenchModule'

vi.mock('../../../lib/tauriGuard', () => ({ safeTauriInvoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(() => Promise.resolve(() => undefined)) }))
const mockedInvoke = vi.mocked(safeTauriInvoke)
const validation = { valid: true, projectId: 'project-1', root: '.', effectivePermissions: ['authority:approval_required', 'network:false'], providerPosture: 'cargo', projectChecks: ['test'], errors: [] }

describe('WorkbenchModule', () => {
  beforeEach(() => { mockedInvoke.mockReset(); window.localStorage.clear() })

  it('resumes the last durable run after a native restart', async () => {
    const run = {
      graph: {
        schema_version: 'arda.run-graph.v1', run_id: 'run-resume-1', objective_id: 'objective-1',
        nodes: [{
          id: 'close', kind: 'close', state: 'succeeded', authority: 'read_only',
          budget: { max_joules: 25, max_cost_usd: 0 }, retry: { max_attempts: 1 }, timeout_ms: 60_000,
          idempotency_key: 'run-resume-1-close', input_digest: 'objective:objective-1', output_digest: 'receipt:close',
          parent_receipts: ['receipt:review'], checkpoint: { sequence: 1, recovery_token: 'resume-1', checkpoint_digest: 'checkpoint:1' },
        }],
        edges: [], provenance: { project_contract_digest: 'project:project-1', created_by: 'test', parent_receipts: [] },
      },
      events: [],
      review: {
        changes: [{ path: 'src/lib.rs', status: 'modified', additions: 2, deletions: 1, diff: '+safe boundary' }],
        tests: [{ name: 'cargo test', status: 'passed', duration_ms: 42, details: 'exit 0' }],
        provider_receipt: { provider: 'nous', model: 'fixture-model', adapter: 'hermes-workbench', receipt_digest: 'sha256:provider', summary: 'Bounded mutation completed.' },
      },
    }
    window.localStorage.setItem('arda.workbench.last-run-id', 'run-resume-1')
    window.localStorage.setItem('arda.workbench.objective.run-resume-1', 'Resume this bounded objective')
    mockedInvoke.mockImplementation((command) => command === 'get_workbench_run' ? Promise.resolve(run) : Promise.resolve(undefined))

    render(<WorkbenchModule />)

    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith('get_workbench_run', { runId: 'run-resume-1' }))
    expect(await screen.findByText(/Resumed run run-resume-1 from the durable harness/)).toBeTruthy()
    expect(screen.getByRole('button', { name: 'close: succeeded' })).toBeTruthy()
    expect(screen.getByText('src/lib.rs')).toBeTruthy(); expect(screen.getByText(/passed \/ 42 ms/)).toBeTruthy()
    expect(screen.getByText('nous / fixture-model')).toBeTruthy()
    expect((screen.getByLabelText('Objective') as HTMLTextAreaElement).value).toBe('Resume this bounded objective')
  })

  it('fails closed on unsafe contract paths and empty objectives', async () => {
    render(<WorkbenchModule />)
    fireEvent.change(screen.getByLabelText('Project contract path'), { target: { value: '../secret/project.json' } })
    fireEvent.click(screen.getByRole('button', { name: 'Validate project contract' }))
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('absolute path'))
    fireEvent.click(screen.getByRole('button', { name: 'Capture objective' }))
    expect(screen.getByRole('alert').textContent).toContain('Objective text is required')
    expect(mockedInvoke).not.toHaveBeenCalled()
  })

  it('shows validation posture before permitting typed attachment', async () => {
    mockedInvoke.mockResolvedValueOnce(validation).mockResolvedValueOnce({ contract: {}, approval_id: 'approval-1', proposal_id: 'proposal-1', idempotency_key: 'attach-1' })
    render(<WorkbenchModule />)
    fireEvent.change(screen.getByLabelText('Project contract path'), { target: { value: '/workspace/project.json' } })
    fireEvent.click(screen.getByRole('button', { name: 'Validate project contract' }))
    const summary = await screen.findByLabelText('Validated project contract')
    expect(within(summary).getByText('authority:approval_required, network:false')).toBeTruthy()
    expect(within(summary).getByText('cargo')).toBeTruthy(); expect(within(summary).getByText('test')).toBeTruthy()
    const attach = screen.getByRole('button', { name: 'Attach project' }); expect((attach as HTMLButtonElement).disabled).toBe(false)
    fireEvent.change(screen.getByLabelText('Proposal ID'), { target: { value: 'proposal-1' } }); fireEvent.change(screen.getByLabelText('Approval ID'), { target: { value: 'approval-1' } }); fireEvent.click(attach)
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith('attach_project_contract', expect.objectContaining({ path: '/workspace/project.json', envelope: expect.any(Object) })))
  })

  it('captures text in the shared objective contract and exposes the approval-first graph', () => {
    render(<WorkbenchModule />)
    fireEvent.change(screen.getByLabelText('Objective'), { target: { value: 'Add one tested parser edge case' } })
    fireEvent.click(screen.getByRole('button', { name: 'Capture objective' }))
    expect(screen.getByRole('button', { name: 'approval: pending' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'review: pending' })).toBeTruthy()
    expect(screen.getByText(/arda.workbench.objective.v1/)).toBeTruthy()
    expect(screen.getByText('No changes recorded.')).toBeTruthy(); expect(screen.getByText('No tests have run.')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'approval: pending' }))
    const review = screen.getByLabelText('Selected run node review')
    expect(within(review).getByText('approval · approval')).toBeTruthy()
    expect(within(review).getByText('pending · human approval')).toBeTruthy()
  })

  it('rejects a durable approval through the typed cancel boundary and keeps revision explicit', async () => {
    const approval = {
      id: 'approval', kind: 'approval', state: 'blocked', authority: 'human_approval',
      budget: { max_joules: 25, max_cost_usd: 0 }, retry: { max_attempts: 1 }, timeout_ms: 60_000,
      idempotency_key: 'run-reject-1-approval', input_digest: 'objective:objective-reject-1', output_digest: null,
      parent_receipts: ['receipt:plan'], checkpoint: { sequence: 1, recovery_token: 'resume-reject-1', checkpoint_digest: 'checkpoint:reject-1' },
    }
    const run = {
      graph: { schema_version: 'arda.run-graph.v1', run_id: 'run-reject-1', objective_id: 'objective-reject-1', nodes: [approval], edges: [], provenance: { project_contract_digest: 'project:project-1', created_by: 'test', parent_receipts: [] } },
      events: [], review: { changes: [], tests: [], provider_receipt: null },
    }
    const cancelled = { ...run, graph: { ...run.graph, nodes: [{ ...approval, state: 'cancelled' }] } }
    window.localStorage.setItem('arda.workbench.last-run-id', 'run-reject-1')
    window.localStorage.setItem('arda.workbench.objective.run-reject-1', 'Original objective')
    window.localStorage.setItem('arda.workbench.proposal.run-reject-1', 'proposal-reject-1')
    window.localStorage.setItem('arda.workbench.approval.run-reject-1', 'approval-reject-1')
    mockedInvoke.mockImplementation((command) => {
      if (command === 'get_workbench_run') return Promise.resolve(run)
      if (command === 'cancel_workbench_run') return Promise.resolve(cancelled)
      return Promise.resolve(undefined)
    })
    render(<WorkbenchModule />)
    fireEvent.click(await screen.findByRole('button', { name: 'Reject' }))
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith('cancel_workbench_run', { request: expect.objectContaining({ run_id: 'run-reject-1', reason: expect.stringContaining('revise the objective') }) }))
    expect(await screen.findByText(/Approval approval rejected\. Revise the objective/)).toBeTruthy()
  })
})
