import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { safeTauriInvoke } from '../../../lib/tauriGuard'
import WorkbenchModule from './WorkbenchModule'

vi.mock('../../../lib/tauriGuard', () => ({ safeTauriInvoke: vi.fn() }))
const mockedInvoke = vi.mocked(safeTauriInvoke)
const validation = { valid: true, projectId: 'project-1', root: '.', effectivePermissions: ['authority:approval_required', 'network:false'], providerPosture: 'cargo', projectChecks: ['test'], errors: [] }

describe('WorkbenchModule', () => {
  beforeEach(() => mockedInvoke.mockReset())

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
    expect(screen.getByText(/arda.workbench.objective.v1/)).toBeTruthy()
    expect(screen.getByText('No changes recorded.')).toBeTruthy(); expect(screen.getByText('No tests have run.')).toBeTruthy()
  })
})
