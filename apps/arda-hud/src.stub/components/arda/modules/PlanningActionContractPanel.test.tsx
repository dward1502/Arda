// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import PlanningActionContractPanel, { buildPlanningTaskPivotPreview } from './PlanningActionContractPanel'
import { getSystemActionCapabilityStatuses, getSystemActionDescriptors } from '../../../lib/systemActionBus'

describe('PlanningActionContractPanel', () => {
  it('builds an operator-gated task-pivot command preview', () => {
    const preview = buildPlanningTaskPivotPreview({
      title: 'ARDA planning lane action contract',
      glyph: '↝',
    })

    expect(preview.command).toContain('cargo run -p arda-cli -- utility task-pivot')
    expect(preview.command).toContain("'ARDA planning lane action contract'")
    expect(preview.command).toContain('--owner prometheus')
    expect(preview.command).toContain("--glyph '↝'")
    expect(preview.command).toContain('--status queued')
  })

  it('exposes queue preview as runnable while task pivot remains governed', () => {
    const onRunAction = vi.fn()
    render(
      <PlanningActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses({
          queueSummary: {
            generated_at_utc: '2026-06-01T06:00:00Z',
          },
        })}
        onRunAction={onRunAction}
      />,
    )

    const contract = screen.getByLabelText('Planning workstation action contract')

    expect(within(contract).getByText('Planning Action Contract')).toBeTruthy()
    expect(within(contract).getByText('Preview Queue Cleanup')).toBeTruthy()
    expect(within(contract).getByText('dry_run')).toBeTruthy()
    expect(within(contract).getByRole('button', { name: /Run Queue Preview/ })).toBeTruthy()
    expect(within(contract).getByText('Capture Task Pivot')).toBeTruthy()
    expect(within(contract).getByText('governed_mutation')).toBeTruthy()
    expect(within(contract).getByText('operator_intent_required')).toBeTruthy()
    expect(within(contract).getByText('Task Pivot Record Preview')).toBeTruthy()
    expect(within(contract).getByText(/governed mutation remains operator-gated/)).toBeTruthy()

    fireEvent.click(within(contract).getByRole('button', { name: /Run Queue Preview/ }))
    expect(onRunAction).toHaveBeenCalledWith('arda.queue_preview_cleanup')
  })

  it('shows busy state for the queue preview action', () => {
    render(
      <PlanningActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses()}
        busyActionId="arda.queue_preview_cleanup"
        message="queue cleanup preview refreshed"
        onRunAction={vi.fn()}
      />,
    )

    const contract = screen.getByLabelText('Planning workstation action contract')
    expect(within(contract).getByRole('button', { name: /Running Preview/ })).toBeDisabled()
    expect(within(contract).getByText('queue cleanup preview refreshed')).toBeTruthy()
  })
})
