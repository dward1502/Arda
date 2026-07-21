// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import WorldTerminalActionContractPanel, {
  buildWorldTerminalActionContracts,
  buildWorldTerminalActionDetails,
} from './WorldTerminalActionContractPanel'
import { getSystemActionCapabilityStatuses, getSystemActionDescriptors } from '../../../lib/systemActionBus'

describe('WorldTerminalActionContractPanel', () => {
  it('defines queue, tools, and status terminal contracts', () => {
    const contracts = buildWorldTerminalActionContracts()

    expect(contracts.map((contract) => contract.terminalId)).toEqual([
      'terminal_queue',
      'terminal_tools',
      'terminal_status',
    ])
    expect(contracts.find((contract) => contract.terminalId === 'terminal_queue')?.safeActionIds).toContain('arda.queue_preview_cleanup')
    expect(contracts.find((contract) => contract.terminalId === 'terminal_tools')?.governedActionIds).toContain('arda.setup_run_repair_flow')
    expect(contracts.find((contract) => contract.terminalId === 'terminal_status')?.safeActionIds).toContain('arda.manwe_refresh_provider_intelligence')
  })

  it('resolves terminal actions into detailed status and receipt evidence', () => {
    const [queueContract] = buildWorldTerminalActionContracts()
    const details = buildWorldTerminalActionDetails(
      queueContract,
      getSystemActionDescriptors(),
      getSystemActionCapabilityStatuses({
        queueSummary: { generated_at_utc: '2026-06-01T06:00:00Z' },
      }),
    )

    expect(details.map((detail) => detail.actionId)).toEqual(['arda.queue_preview_cleanup', 'arda.queue_capture_pivot'])
    expect(details[0]).toMatchObject({
      label: 'Preview Queue Cleanup',
      currentStatus: 'succeeded',
      riskLevel: 'dry_run',
      receiptPath: 'core/state/queue_summary.json',
    })
    expect(details[1]).toMatchObject({
      label: 'Capture Task Pivot',
      currentStatus: 'blocked',
      governed: true,
      governanceGate: 'operator_intent_required',
    })
  })

  it('renders runnable safe terminal actions and preview-only governed actions', () => {
    const onRunAction = vi.fn()
    render(
      <WorldTerminalActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses({
          queueSummary: { generated_at_utc: '2026-06-01T06:00:00Z' },
          setupConsoleReadiness: { gate_status: 'pass', generated_at_utc: '2026-06-01T06:01:00Z' },
          repeatedAuditStatus: { gate_status: 'pass', generated_at_utc: '2026-06-01T06:02:00Z' },
          providerIntelligence: { generated_at_utc: '2026-06-01T06:03:00Z' },
          chronosRuntime: { status: 'ok', generated_at_utc: '2026-06-01T06:04:00Z' },
        })}
        onRunAction={onRunAction}
      />,
    )

    const panel = screen.getByLabelText('World terminal action contracts')
    expect(within(panel).getByText('Queue Terminal')).toBeTruthy()
    expect(within(panel).getByText('Tools Terminal')).toBeTruthy()
    expect(within(panel).getByText('Status Terminal')).toBeTruthy()
    expect(within(panel).getByText(/Run Setup Repair Flow: blocked/)).toBeTruthy()
    expect(within(panel).getAllByText('core/state/queue_summary.json').length).toBeGreaterThanOrEqual(1)
    expect(within(panel).getByText('operator_approval_required_for_repair')).toBeTruthy()
    expect(within(panel).getByText('provider_metadata_refresh_only')).toBeTruthy()

    fireEvent.click(within(panel).getByRole('button', { name: /Preview Queue Cleanup/ }))
    fireEvent.click(within(panel).getByRole('button', { name: /Run Setup Readiness Check/ }))
    fireEvent.click(within(panel).getByRole('button', { name: /Refresh Provider Intelligence/ }))

    expect(onRunAction).toHaveBeenCalledWith('arda.queue_preview_cleanup')
    expect(onRunAction).toHaveBeenCalledWith('arda.setup_run_readiness_check')
    expect(onRunAction).toHaveBeenCalledWith('arda.manwe_refresh_provider_intelligence')
  })

  it('shows busy state for a selected terminal action', () => {
    render(
      <WorldTerminalActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses()}
        busyActionId="arda.setup_run_readiness_check"
        message="setup readiness refreshed"
        onRunAction={vi.fn()}
      />,
    )

    const panel = screen.getByLabelText('World terminal action contracts')
    expect(within(panel).getByRole('button', { name: /Running/ })).toBeDisabled()
    expect(within(panel).getByText('setup readiness refreshed')).toBeTruthy()
  })
})
