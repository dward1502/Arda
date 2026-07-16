// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import KnowledgeActionContractPanel, { buildKnowledgeActionContract } from './KnowledgeActionContractPanel'
import { getSystemActionCapabilityStatuses, getSystemActionDescriptors } from '../../../../lib/systemActionBus'

describe('KnowledgeActionContractPanel', () => {
  it('defines safe ATHENA refresh and governed knowledge mutations', () => {
    const contract = buildKnowledgeActionContract()

    expect(contract.safeActionIds).toEqual(['arda.athena_refresh_digest'])
    expect(contract.governedActionIds).toEqual(['arda.athena_ingest_knowledge', 'arda.athena_promote_policy_ready'])
    expect(contract.evidencePaths).toContain('core/state/knowledge_triage_registry.jsonl')
  })

  it('runs safe digest refresh and governed policy preview without exposing ingestion mutation', () => {
    const onRunAction = vi.fn()
    render(
      <KnowledgeActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses({
          athenaRuntime: { status: 'ok', generated_at_utc: '2026-06-01T06:00:00Z' },
          knowledgeTriage: { generated_at_utc: '2026-06-01T06:01:00Z' },
        })}
        policySummary={{
          status: 'review_pressure',
          policyReadyTotal: 3,
          referenceOnlyTotal: 8,
          reviewPressureTotal: 5,
          nextOperatorAction: 'preview_policy_ready_promotion',
          promotionPreviewAvailable: true,
          governanceGate: 'human_review_required',
        }}
        onRunAction={onRunAction}
      />,
    )

    const panel = screen.getByLabelText('Knowledge action contract')
    expect(within(panel).getByText('Knowledge Action Contract')).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Refresh ATHENA Digest/ })).toBeTruthy()
    expect(within(panel).getByText(/5 blocked \/ 3 ready \/ 8 reference-only/)).toBeTruthy()
    expect(within(panel).getByText(/preview_policy_ready_promotion/)).toBeTruthy()
    expect(within(panel).getByText(/Ingest Knowledge Notes: blocked/)).toBeTruthy()
    expect(within(panel).getByText(/Promote Policy-Ready Knowledge: blocked/)).toBeTruthy()
    expect(within(panel).queryByRole('button', { name: /Preview Ingest Knowledge Notes/ })).toBeNull()
    expect(within(panel).getByRole('button', { name: /Preview Promote Policy-Ready Knowledge/ })).toBeTruthy()

    fireEvent.click(within(panel).getByRole('button', { name: /Refresh ATHENA Digest/ }))
    fireEvent.click(within(panel).getByRole('button', { name: /Preview Promote Policy-Ready Knowledge/ }))
    expect(onRunAction).toHaveBeenNthCalledWith(1, 'arda.athena_refresh_digest')
    expect(onRunAction).toHaveBeenNthCalledWith(2, 'arda.athena_promote_policy_ready')
  })

  it('shows busy state for digest refresh', () => {
    render(
      <KnowledgeActionContractPanel
        actionDescriptors={getSystemActionDescriptors()}
        capabilityStatuses={getSystemActionCapabilityStatuses()}
        busyActionId="arda.athena_refresh_digest"
        message="ATHENA digest refreshed"
        onRunAction={vi.fn()}
      />,
    )

    const panel = screen.getByLabelText('Knowledge action contract')
    expect(within(panel).getByRole('button', { name: /Running/ })).toBeDisabled()
    expect(within(panel).getByText('ATHENA digest refreshed')).toBeTruthy()
  })
})
