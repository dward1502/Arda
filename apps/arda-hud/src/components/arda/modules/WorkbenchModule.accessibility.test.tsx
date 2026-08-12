import { fireEvent, render, screen } from '@testing-library/react'
import axe from 'axe-core'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { safeTauriInvoke } from '../../../lib/tauriGuard'
import WorkbenchModule, { summarizeOperatorState } from './WorkbenchModule'

vi.mock('../../../lib/tauriGuard', () => ({ safeTauriInvoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(() => Promise.resolve(() => undefined)) }))

const mockedInvoke = vi.mocked(safeTauriInvoke)

async function expectNoBlockingViolations(container: HTMLElement) {
  const result = await axe.run(container, {
    rules: {
      // jsdom has no layout/paint engine; contrast is covered by native visual acceptance.
      'color-contrast': { enabled: false },
    },
  })
  const blocking = result.violations.filter(({ impact }) => impact === 'critical' || impact === 'serious')
  expect(blocking.map(({ id, nodes }) => ({ id, targets: nodes.map(({ target }) => target) }))).toEqual([])
}

describe('WorkbenchModule accessibility gate', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
    window.localStorage.clear()
  })

  it('has no critical or serious automated violations in initial and objective states', async () => {
    const { container } = render(<WorkbenchModule />)
    await expectNoBlockingViolations(container)

    fireEvent.change(screen.getByLabelText('Objective'), { target: { value: 'Verify one accessible recovery path' } })
    fireEvent.click(screen.getByRole('button', { name: 'Capture objective' }))
    expect(screen.getByRole('status').textContent).toContain('Rust will create the governed run graph')
    await expectNoBlockingViolations(container)
  })

  it('keeps every initial Workbench control keyboard-focusable in document order', () => {
    const { container } = render(<WorkbenchModule />)
    const controls = Array.from(container.querySelectorAll<HTMLElement>(
      'button:not(:disabled), input:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
    ))
    expect(controls.length).toBeGreaterThan(5)
    for (const control of controls) {
      control.focus()
      expect(document.activeElement).toBe(control)
    }
    expect(screen.getByRole('status').getAttribute('aria-live')).toBe('polite')
  })

  it('answers the four operator-state questions in plain language', () => {
    render(<WorkbenchModule />)
    expect(screen.getByText('What happened?')).toBeTruthy()
    expect(screen.getByText('Why?')).toBeTruthy()
    expect(screen.getByText('What can act?')).toBeTruthy()
    expect(screen.getByText('What evidence is available?')).toBeTruthy()
    expect(screen.getByText('What should I do next?')).toBeTruthy()
    expect(screen.getByText('Validate a project contract before attaching it.')).toBeTruthy()
    expect(screen.getByText('No execution receipt or project verification evidence exists yet.')).toBeTruthy()
  })

  it('explains failed-run authority, reason, and recovery action', () => {
    const graph = {
      schema_version: 'arda.run-graph.v1' as const,
      run_id: 'run-1', objective_id: 'objective-1', edges: [],
      provenance: { project_contract_digest: 'sha256:test', created_by: 'operator', parent_receipts: [] },
      nodes: [{
        id: 'execute-1', kind: 'execute' as const, state: 'failed' as const,
        authority: 'execute_with_approval' as const, budget: { max_joules: 1, max_cost_usd: 0 },
        retry: { max_attempts: 1 }, timeout_ms: 1000, idempotency_key: 'execute-1',
        input_digest: null, output_digest: null, parent_receipts: [],
        checkpoint: { sequence: 0, recovery_token: null, checkpoint_digest: null },
      }],
    }
    const summary = summarizeOperatorState({
      graph, events: [{ reason: 'Provider timed out before producing a receipt.' }],
      error: null, message: 'Run updated.', validationValid: true, attached: true,
      objectivePresent: true, runPresent: true,
    })
    expect(summary.whatHappened).toContain('execute step')
    expect(summary.why).toContain('Provider timed out')
    expect(summary.whatCanAct).toContain('only after operator approval')
    expect(summary.evidenceQuality).toContain('Project success is not proven')
    expect(summary.nextAction).toContain('revise or recover')
  })
})
