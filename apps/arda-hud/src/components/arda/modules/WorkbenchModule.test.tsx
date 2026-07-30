import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { safeTauriInvoke } from '../../../lib/tauriGuard'
import WorkbenchModule from './WorkbenchModule'

vi.mock('../../../lib/tauriGuard', () => ({
  safeTauriInvoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(safeTauriInvoke)

describe('WorkbenchModule', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
  })

  it('fails closed on unsafe contract paths and empty objectives', () => {
    render(<WorkbenchModule />)
    fireEvent.change(screen.getByLabelText('Project contract path'), {
      target: { value: '../secret/project.json' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Prepare governed run' }))

    expect(screen.getByRole('alert').textContent).toContain('absolute path')
    expect(screen.queryByLabelText('Workbench run graph')).toBeNull()
  })

  it('shows a bounded approval-first graph without claiming execution', () => {
    render(<WorkbenchModule />)
    fireEvent.change(screen.getByLabelText('Project contract path'), {
      target: { value: '/workspace/spec/project-contract/v1/examples/rust-project.json' },
    })
    fireEvent.change(screen.getByLabelText('Objective'), {
      target: { value: 'Add one tested parser edge case' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Prepare governed run' }))

    const graph = screen.getByLabelText('Workbench run graph')
    expect(within(graph).getByText('inspect')).toBeTruthy()
    expect(within(graph).getByText('approval')).toBeTruthy()
    expect(within(graph).getByText('execute')).toBeTruthy()
    expect(within(graph).getAllByText('verify').length).toBeGreaterThan(0)
    expect(screen.getByText('approval required')).toBeTruthy()
    expect(screen.getByText(/draft only; no project was attached/)).toBeTruthy()
  })

  it('validates a project contract through the native boundary before attachment', async () => {
    mockedInvoke.mockResolvedValue({
      schemaVersion: 'arda.project-contract.v1',
      projectId: '550e8400-e29b-41d4-a716-446655440000',
      name: 'arda-rust-example',
      kind: 'rust',
      workspaceRoot: '.',
      runtimeAdapter: 'cargo',
      commandIds: ['test'],
      checkIds: ['test'],
      permissions: {
        authority: 'approval_required',
        networkAllowed: false,
        filesystemWrite: true,
        secretEnvNames: [],
      },
    })
    render(<WorkbenchModule />)
    const contractPath = '/workspace/spec/project-contract/v1/examples/rust-project.json'
    fireEvent.change(screen.getByLabelText('Project contract path'), {
      target: { value: contractPath },
    })

    fireEvent.click(screen.getByRole('button', { name: 'Validate project contract' }))

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('validate_project_contract', { path: contractPath })
    })
    const summary = screen.getByLabelText('Validated project contract')
    expect(within(summary).getByText('arda-rust-example')).toBeTruthy()
    expect(within(summary).getByText('approval_required')).toBeTruthy()
    expect(within(summary).getByText('network denied')).toBeTruthy()
    expect(within(summary).getByText(/validated only; project is not attached/)).toBeTruthy()
  })
})
