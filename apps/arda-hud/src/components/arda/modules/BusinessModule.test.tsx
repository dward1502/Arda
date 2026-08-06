import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import BusinessModule from './BusinessModule'
import type { CompanyOpsSnapshot } from '../../../lib/companyOps'

const snapshot: CompanyOpsSnapshot = {
  engagements: [{ engagement_id: 'engagement-1', title: 'Paid discovery', state: 'won', expected_value: { currency: 'USD', low: 800, expected: 1000, high: 1400, confidence: 0.65 }, realized_value: null }],
  opportunities: [{
    opportunity_id: 'op-1', title: 'Paid discovery', stage: 'qualified',
    expected_value: { currency: 'USD', low: 800, expected: 1000, high: 1400, confidence: 0.65 },
    score: 0.76, score_components: { urgency: 0.8, evidence_quality: 0.65 },
    evidence: [{ source_id: 'crm-1', citation: 'CRM opportunity 1', assumption: 'Client still needs discovery' }],
    next_action: 'Review and approve discovery proposal',
  }],
  commitments: [{ commitment_id: 'commit-1', title: 'Deliver prototype', due_at: '2026-08-08T12:00:00Z', approval_receipt_id: 'receipt-1', status: 'approved' }],
  experiments: [{ experiment_id: 'exp-1', title: 'Paid discovery offer', status: 'running', success_threshold: 'one paid engagement', stop_condition: 'four hours without reply', operator_hours_used: 1, operator_hours_max: 4 }],
  drafts: [{ proposal_id: 'draft-1', title: 'Discovery SOW', audience: 'client', approval_required: true }],
  expected_value: { currency: 'USD', low: 800, expected: 1000, high: 1400, confidence: 0.65 },
  realized_value: { currency: 'USD', amount: 0, outcome_receipt_id: 'none' },
  cost: { currency: 'USD', amount: 30, operator_hours: 1 },
}

describe('Company Operations Business module', () => {
  it('shows governed next action, pipeline, commitments, experiments, drafts, and value evidence', () => {
    render(<BusinessModule mode="active" clientCount={1} stateKeyCount={1} companyViewTitle="Company View" companyViewPreview="Current state" clientPaths={[]} stateKeys={[]} companyOps={snapshot} />)
    expect(screen.getByText('Highest-value next operator action')).toBeInTheDocument()
    expect(screen.getByText('Review and approve discovery proposal')).toBeInTheDocument()
    expect(screen.getByText('Commitments due soon')).toBeInTheDocument()
    expect(screen.getByText('Opportunity board')).toBeInTheDocument()
    expect(screen.getByText('Active paid and client work')).toBeInTheDocument()
    expect(screen.getByText('Revenue experiments')).toBeInTheDocument()
    expect(screen.getByText('Drafts awaiting approval')).toBeInTheDocument()
    expect(screen.getByText('Expected versus realized value')).toBeInTheDocument()
    expect(screen.getByText(/Assumptions: Client still needs discovery/)).toBeInTheDocument()
    expect(screen.getByText(/Explicit operator approval required/)).toBeInTheDocument()
  })
})
