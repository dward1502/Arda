// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import OperationsActionContractPanel, { buildOperationsActionContracts } from './OperationsActionContractPanel'
import { getSystemActionCapabilityStatuses, getSystemActionDescriptors } from '../../../lib/systemActionBus'

describe('OperationsActionContractPanel', () => {
  it('maps operations lanes to safe and governed action contracts', () => {
    const contracts = buildOperationsActionContracts()

    expect(contracts).toHaveLength(3)
    expect(contracts.find((contract) => contract.laneId === 'hades_maintenance')?.safeActionIds).toContain('arda.hades_run_link_check')
    expect(contracts.find((contract) => contract.laneId === 'audit_evidence')?.safeActionIds).toContain('arda.audit_run_repeated_audit')
    expect(contracts.find((contract) => contract.laneId === 'setup_readiness')?.governedActionIds).toContain('arda.setup_run_repair_flow')
  })

  it('exposes read-only and dry-run operations as runnable while repair remains governed', () => {
    const onRunAction = vi.fn()
    render(
      <OperationsActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses({
          hadesNightlyOperations: {
            status: 'pass',
            generated_at_utc: '2026-06-01T06:00:00Z',
          },
          repeatedAuditStatus: {
            gate_status: 'pass',
            generated_at_utc: '2026-06-01T06:05:00Z',
          },
          setupConsoleReadiness: {
            gate_status: 'pass',
            generated_at_utc: '2026-06-01T06:10:00Z',
          },
        })}
        onRunAction={onRunAction}
      />,
    )

    const panel = screen.getByLabelText('Operations action contracts')
    expect(within(panel).getByText('Operations Action Contracts')).toBeTruthy()
    expect(within(panel).getByText('HADES Maintenance')).toBeTruthy()
    expect(within(panel).getByText('Audit Evidence')).toBeTruthy()
    expect(within(panel).getByText('Setup Readiness')).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Preview Organization Plan/ })).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Run Repeated Audit/ })).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Run Setup Readiness Check/ })).toBeTruthy()
    expect(within(panel).getByText(/Run Setup Repair Flow: blocked/)).toBeTruthy()
    expect(within(panel).getByText(/operator_approval_required_for_repair/)).toBeTruthy()

    fireEvent.click(within(panel).getByRole('button', { name: /Run Link Check/ }))
    expect(onRunAction).toHaveBeenCalledWith('arda.hades_run_link_check')
  })

  it('shows busy state for an operations action', () => {
    render(
      <OperationsActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses()}
        busyActionId="arda.audit_run_repeated_audit"
        message="repeated audit preview refreshed"
        onRunAction={vi.fn()}
      />,
    )

    const panel = screen.getByLabelText('Operations action contracts')
    expect(within(panel).getByRole('button', { name: /Running/ })).toBeDisabled()
    expect(within(panel).getByText('repeated audit preview refreshed')).toBeTruthy()
  })
})
