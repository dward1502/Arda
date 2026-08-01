import { fireEvent, render, screen } from '@testing-library/react'
import axe from 'axe-core'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { safeTauriInvoke } from '../../../lib/tauriGuard'
import WorkbenchModule from './WorkbenchModule'

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
    expect(screen.getByRole('button', { name: 'approval: pending' })).toBeTruthy()
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
})
