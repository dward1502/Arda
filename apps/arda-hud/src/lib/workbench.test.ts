import { beforeEach, describe, expect, it, vi } from 'vitest'
import { safeTauriInvoke } from './tauriGuard'
import { attachProjectContract, createObjective, planWorkbenchRun } from './workbench'

vi.mock('./tauriGuard', () => ({ safeTauriInvoke: vi.fn() }))
const invoke = vi.mocked(safeTauriInvoke)

beforeEach(() => invoke.mockReset())

describe('Workbench intent boundary', () => {
  it('submits approval reference intent without a browser approval envelope', () => {
    void attachProjectContract('/workspace/project.json', { approvalReference: 'approval-1' })

    expect(invoke).toHaveBeenCalledWith('attach_project_contract', {
      path: '/workspace/project.json',
      intent: { approvalReference: 'approval-1' },
    })
    expect(JSON.stringify(invoke.mock.calls[0])).not.toContain('policy_safe')
  })

  it('submits objective intent without browser IDs, topology, budgets, or provenance', () => {
    const objective = createObjective('Apply one bounded change')
    void planWorkbenchRun('project-1', objective, { approvalReference: 'approval-1' })

    expect(invoke).toHaveBeenCalledWith('plan_workbench_run', {
      request: {
        project_id: 'project-1',
        objective: { text: 'Apply one bounded change', input_mode: 'text' },
        intent: { approvalReference: 'approval-1' },
      },
    })
    const payload = JSON.stringify(invoke.mock.calls[0])
    expect(payload).not.toContain('run_id')
    expect(payload).not.toContain('nodes')
    expect(payload).not.toContain('provenance')
    expect(payload).not.toContain('idempotency_key')
  })
})
