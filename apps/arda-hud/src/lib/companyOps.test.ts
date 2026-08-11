import { describe, expect, it } from 'vitest'
import { companyOpsFromProjection, highestValueAction } from './companyOps'

describe('companyOpsFromProjection', () => {
  it('normalizes canonical Aule projections and preserves score uncertainty', () => {
    const opportunity = {
      opportunity_id: 'opp-1', title: 'Paid cockpit setup', stage: 'qualified',
      expected_value: { currency: 'USD', range: { low: 800, expected: 1000, high: 1400, confidence: 0.65 }, basis: 'comparable work' },
      evidence: [{ source_id: 'source-1', citation: 'receipt:1' }],
    }
    const snapshot = companyOpsFromProjection({
      opportunities: [opportunity],
      scored_opportunities: [{ opportunity, score: { expected_value: 0.1, uncertainty: 0.35, total: 0.73 } }],
      drafts: [{ proposal_id: 'draft-1', title: 'Trial proposal', audience: 'Client', authority: 'explicit_operator_approval' }],
      commitments: [{ commitment_id: 'commitment-1', scope: 'One setup', due_at: '2026-08-10T00:00:00Z', approval_receipt_id: 'approval-1' }],
      engagements: [{ engagement_id: 'engagement-1', title: 'Paid discovery', state: 'paid', expected_value: opportunity.expected_value, realized_value: { currency: 'USD', amount: 1000, outcome_receipt_id: 'outcome-1', realized_at: '2026-08-04T12:00:00Z' } }],
      experiments: [{ experiment_id: 'experiment-1', hypothesis: { proposed_offer: 'Setup trial' }, maximum_operator_time: { maximum_hours: 4 }, maximum_spend: { currency: 'USD', range: { low: 0, expected: 50, high: 100, confidence: 0.8 } }, success_threshold: 'one trial', stop_condition: 'four hours', approval_receipt_id: null }],
      outcomes: [],
    })
    expect(highestValueAction(snapshot)?.title).toBe('Paid cockpit setup')
    expect(snapshot.opportunities[0].score_components.uncertainty).toBe(0.35)
    expect(snapshot.drafts[0].approval_required).toBe(true)
    expect(snapshot.cost).toEqual({ currency: 'USD', amount: 100, operator_hours: 4 })
    expect(snapshot.engagements[0]).toMatchObject({ title: 'Paid discovery', state: 'paid' })
    expect(snapshot.realized_value).toEqual({ currency: 'USD', amount: 1000, outcome_receipt_id: 'outcome-1' })
  })
})
