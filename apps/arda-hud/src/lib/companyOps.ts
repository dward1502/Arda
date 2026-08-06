export type PipelineStage = 'lead' | 'qualified' | 'proposed' | 'won' | 'lost' | 'delivered' | 'invoiced' | 'paid'

export interface ValueRange { currency: string; low: number; expected: number; high: number; confidence: number }
export interface ValueEvidence { source_id: string; citation: string; assumption: string }
export interface CompanyEngagement {
  engagement_id: string
  title: string
  state: PipelineStage
  expected_value: ValueRange
  realized_value: { currency: string; amount: number; outcome_receipt_id: string } | null
}
export interface CompanyOpportunity {
  opportunity_id: string
  title: string
  stage: PipelineStage
  expected_value: ValueRange
  score: number
  score_components: Record<string, number>
  evidence: ValueEvidence[]
  next_action: string
}
export interface CompanyCommitment { commitment_id: string; title: string; due_at: string; approval_receipt_id: string; status: 'approved' | 'delivered' | 'invoiced' | 'paid' }
export interface CompanyExperiment { experiment_id: string; title: string; status: 'proposal' | 'approved' | 'running' | 'continue' | 'pivot' | 'stop'; success_threshold: string; stop_condition: string; operator_hours_used: number; operator_hours_max: number }
export interface CompanyDraft { proposal_id: string; title: string; audience: string; approval_required: boolean }
export interface CompanyOpsSnapshot {
  engagements: CompanyEngagement[]
  opportunities: CompanyOpportunity[]
  commitments: CompanyCommitment[]
  experiments: CompanyExperiment[]
  drafts: CompanyDraft[]
  expected_value: ValueRange
  realized_value: { currency: string; amount: number; outcome_receipt_id: string }
  cost: { currency: string; amount: number; operator_hours: number }
}

export const emptyCompanyOpsSnapshot: CompanyOpsSnapshot = {
  engagements: [], opportunities: [], commitments: [], experiments: [], drafts: [],
  expected_value: { currency: 'USD', low: 0, expected: 0, high: 0, confidence: 0 },
  realized_value: { currency: 'USD', amount: 0, outcome_receipt_id: 'none' },
  cost: { currency: 'USD', amount: 0, operator_hours: 0 },
}

export function highestValueAction(snapshot: CompanyOpsSnapshot): CompanyOpportunity | null {
  return [...snapshot.opportunities].sort((a, b) => b.score - a.score || a.opportunity_id.localeCompare(b.opportunity_id))[0] ?? null
}

export function formatMoney(currency: string, amount: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency, maximumFractionDigits: 0 }).format(amount)
}

type UnknownRecord = Record<string, unknown>

const record = (value: unknown): UnknownRecord => value && typeof value === 'object' && !Array.isArray(value) ? value as UnknownRecord : {}
const list = (value: unknown): unknown[] => Array.isArray(value) ? value : []
const text = (value: unknown, fallback = ''): string => typeof value === 'string' ? value : fallback
const number = (value: unknown): number => typeof value === 'number' && Number.isFinite(value) ? value : 0

function valueRange(value: unknown): ValueRange {
  const estimate = record(value)
  const range = record(estimate.range)
  return {
    currency: text(estimate.currency, 'USD'),
    low: number(range.low),
    expected: number(range.expected),
    high: number(range.high),
    confidence: number(range.confidence),
  }
}

export function companyOpsFromProjection(value: unknown): CompanyOpsSnapshot {
  const projection = record(value)
  const engagements = list(projection.engagements).map((entry): CompanyEngagement => {
    const engagement = record(entry)
    const realized = record(engagement.realized_value)
    return {
      engagement_id: text(engagement.engagement_id),
      title: text(engagement.title, 'Untitled engagement'),
      state: text(engagement.state, 'lead') as PipelineStage,
      expected_value: valueRange(engagement.expected_value),
      realized_value: engagement.realized_value == null ? null : {
        currency: text(realized.currency, 'USD'),
        amount: number(realized.amount),
        outcome_receipt_id: text(realized.outcome_receipt_id),
      },
    }
  })
  const scoredById = new Map(list(projection.scored_opportunities).map((entry) => {
    const scored = record(entry)
    return [text(record(scored.opportunity).opportunity_id), record(scored.score)] as const
  }))
  const opportunities = list(projection.opportunities).map((entry): CompanyOpportunity => {
    const opportunity = record(entry)
    const score = scoredById.get(text(opportunity.opportunity_id)) ?? {}
    const components = Object.fromEntries(Object.entries(score)
      .filter(([key, item]) => key !== 'total' && typeof item === 'number')) as Record<string, number>
    return {
      opportunity_id: text(opportunity.opportunity_id),
      title: text(opportunity.title, 'Untitled opportunity'),
      stage: text(opportunity.stage, 'lead') as PipelineStage,
      expected_value: valueRange(opportunity.expected_value),
      score: number(score.total),
      score_components: components,
      evidence: list(opportunity.evidence).map((item) => {
        const evidence = record(item)
        return {
          source_id: text(evidence.source_id),
          citation: text(evidence.citation),
          assumption: text(record(opportunity.expected_value).basis),
        }
      }),
      next_action: `Review evidence and advance or retire ${text(opportunity.title, 'this opportunity')}.`,
    }
  })
  const commitments = list(projection.commitments).map((entry): CompanyCommitment => {
    const commitment = record(entry)
    return {
      commitment_id: text(commitment.commitment_id),
      title: text(commitment.scope, 'Approved commitment'),
      due_at: text(commitment.due_at),
      approval_receipt_id: text(commitment.approval_receipt_id),
      status: 'approved',
    }
  })
  const experiments = list(projection.experiments).map((entry): CompanyExperiment => {
    const experiment = record(entry)
    const hypothesis = record(experiment.hypothesis)
    const maximumTime = record(experiment.maximum_operator_time)
    return {
      experiment_id: text(experiment.experiment_id),
      title: text(hypothesis.proposed_offer, 'Bounded experiment'),
      status: text(experiment.decision, experiment.approval_receipt_id ? 'approved' : 'proposal') as CompanyExperiment['status'],
      success_threshold: text(experiment.success_threshold),
      stop_condition: text(experiment.stop_condition),
      operator_hours_used: 0,
      operator_hours_max: number(maximumTime.maximum_hours),
    }
  })
  const drafts = list(projection.drafts).map((entry): CompanyDraft => {
    const draft = record(entry)
    return {
      proposal_id: text(draft.proposal_id),
      title: text(draft.title, 'Proposal draft'),
      audience: text(draft.audience),
      approval_required: text(draft.authority) !== 'read_only',
    }
  })
  const expectedValue = opportunities.reduce((sum, opportunity) => ({
    currency: opportunity.expected_value.currency,
    low: sum.low + opportunity.expected_value.low,
    expected: sum.expected + opportunity.expected_value.expected,
    high: sum.high + opportunity.expected_value.high,
    confidence: opportunities.length === 0 ? 0 : sum.confidence + opportunity.expected_value.confidence / opportunities.length,
  }), { ...emptyCompanyOpsSnapshot.expected_value })
  const costs = experiments.map((experiment, index) => ({
    hours: experiment.operator_hours_max,
    estimate: valueRange(record(list(projection.experiments)[index]).maximum_spend),
  }))
  const realizedValues = engagements
    .map((engagement) => engagement.realized_value)
    .filter((value): value is NonNullable<CompanyEngagement['realized_value']> => value !== null)
  const realizedCurrency = realizedValues[0]?.currency ?? 'USD'
  return {
    engagements,
    opportunities,
    commitments,
    experiments,
    drafts,
    expected_value: expectedValue,
    realized_value: {
      currency: realizedCurrency,
      amount: realizedValues
        .filter((value) => value.currency === realizedCurrency)
        .reduce((sum, value) => sum + value.amount, 0),
      outcome_receipt_id: realizedValues.map((value) => value.outcome_receipt_id).join(', ') || 'none',
    },
    cost: {
      currency: costs[0]?.estimate.currency ?? 'USD',
      amount: costs.reduce((sum, item) => sum + item.estimate.high, 0),
      operator_hours: costs.reduce((sum, item) => sum + item.hours, 0),
    },
  }
}
