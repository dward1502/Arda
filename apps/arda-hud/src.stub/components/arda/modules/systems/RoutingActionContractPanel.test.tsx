// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { getSystemActionCapabilityStatuses, getSystemActionDescriptors } from '../../../../lib/systemActionBus'
import RoutingActionContractPanel, { buildRoutingActionContract } from './RoutingActionContractPanel'

describe('RoutingActionContractPanel', () => {
  it('defines safe routing refresh actions and keeps route mutation gated', () => {
    const contract = buildRoutingActionContract()

    expect(contract.safeActionIds).toEqual([
      'arda.chronos_run_provider_checks',
      'arda.charon_refresh_provider_intelligence',
    ])
    expect(contract.governedMutation.status).toBe('not exposed')
    expect(contract.evidencePaths).toContain('core/state/charon_router.json')
  })

  it('runs safe provider refresh actions only', () => {
    const onRunAction = vi.fn()
    render(
      <RoutingActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses({
          chronosRuntime: { status: 'ok', generated_at_utc: '2026-06-01T06:00:00Z' },
          providerIntelligence: { generated_at_utc: '2026-06-01T06:01:00Z' },
        })}
        onRunAction={onRunAction}
      />,
    )

    const panel = screen.getByLabelText('Routing action contract')
    expect(within(panel).getByText('Routing Action Contract')).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Run CHRONOS Provider Checks/ })).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Refresh Provider Intelligence/ })).toBeTruthy()
    expect(within(panel).getByText(/Provider reroute: not exposed/)).toBeTruthy()

    fireEvent.click(within(panel).getByRole('button', { name: /Refresh Provider Intelligence/ }))
    expect(onRunAction).toHaveBeenCalledWith('arda.charon_refresh_provider_intelligence')
  })

  it('shows busy state for provider checks', () => {
    render(
      <RoutingActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses()}
        busyActionId="arda.chronos_run_provider_checks"
        message="CHRONOS provider checks refreshed"
        onRunAction={vi.fn()}
      />,
    )

    const panel = screen.getByLabelText('Routing action contract')
    expect(within(panel).getByRole('button', { name: /Running/ })).toBeDisabled()
    expect(within(panel).getByText('CHRONOS provider checks refreshed')).toBeTruthy()
  })
})
