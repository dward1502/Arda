// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import KnowledgeMapPanel from './KnowledgeMapPanel'

const registryEntry = {
  path: 'apps/arda-hud/ARDA_IMPLEMENTATION_PLAN.md',
  title: 'ARDA Implementation Plan',
  classification: 'active',
  canonicalHome: 'apps/arda-hud',
  authority: 'canonical',
  domain: 'arda_control_surface',
  glyph: '🛠',
}

const digestEntry = {
  sourceId: 'src_digest_demo',
  title: 'Policy research packet',
  status: 'shallow',
  sourceType: 'raw_note',
  tags: ['policy', 'athena'],
  summary: 'Initial shallow ingest completed for policy research.',
}

const deepGraphEntry = {
  sourceId: 'src_graph_demo',
  confidence: '0.82',
  triadPassed: true,
  nodeCount: 3,
  edgeCount: 2,
  labels: ['policy packet', 'tag:athena'],
}

const readinessEntry = {
  sourceId: 'src_ready_demo',
  readiness: 'policy_ready',
  confidence: '0.91',
  blockers: [],
  triadPassed: true,
}

const policySummary = {
  status: 'review_pressure',
  policyReadyTotal: 3,
  referenceOnlyTotal: 8,
  reviewPressureTotal: 5,
  nextOperatorAction: 'preview_policy_ready_promotion',
  promotionPreviewAvailable: true,
  governanceGate: 'human_review_required',
}

const missingProjectionEntry = {
  sectionId: 'knowledge_and_reasoning',
  sectionTitle: 'Knowledge And Reasoning',
  owner: 'athena',
  missing: ['human_library_projection_for_boardroom_consumption'],
}

describe('KnowledgeMapPanel', () => {
  it('separates registry, digest, deep graph, and policy readiness views', () => {
    render(
      <KnowledgeMapPanel
        summary={[{ label: 'Total', value: '1' }]}
        entries={[registryEntry]}
        digest={[digestEntry]}
        deepGraph={[deepGraphEntry]}
        policyReadiness={[readinessEntry]}
        policySummary={policySummary}
        missingProjections={[]}
      />,
    )

    expect(screen.getByRole('heading', { name: /Knowledge Map/i })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /Registry 1/i })).toHaveAttribute('aria-selected', 'true')
    const registryPanel = screen.getByRole('tabpanel', { name: /Registry/i })
    expect(within(registryPanel).getByText((_, element) => element?.tagName === 'STRONG' && element.textContent?.includes('ARDA Implementation Plan') === true)).toBeInTheDocument()

    fireEvent.click(screen.getByRole('tab', { name: /Digest 1/i }))
    expect(screen.getByRole('tab', { name: /Digest 1/i })).toHaveAttribute('aria-selected', 'true')
    const digestPanel = screen.getByRole('tabpanel', { name: /Digest/i })
    expect(within(digestPanel).getByText('Policy research packet')).toBeInTheDocument()
    expect(within(digestPanel).getByText(/policy, athena/i)).toBeInTheDocument()

    fireEvent.click(screen.getByRole('tab', { name: /Deep Graph 1/i }))
    const graphPanel = screen.getByRole('tabpanel', { name: /Deep Graph/i })
    expect(within(graphPanel).getByText(/src_graph_demo/i)).toBeInTheDocument()
    expect(within(graphPanel).getByText(/nodes 3/i)).toBeInTheDocument()

    fireEvent.click(screen.getByRole('tab', { name: /Policy Readiness 1/i }))
    const readinessPanel = screen.getByRole('tabpanel', { name: /Policy Readiness/i })
    expect(within(readinessPanel).getByText(/policy_ready/i)).toBeInTheDocument()
    expect(within(readinessPanel).getByText(/no blockers/i)).toBeInTheDocument()
  })

  it('surfaces missing source-map projections as a coverage warning view', () => {
    render(
      <KnowledgeMapPanel
        summary={[{ label: 'Total', value: '0' }]}
        entries={[]}
        digest={[]}
        deepGraph={[]}
        policyReadiness={[]}
        policySummary={policySummary}
        missingProjections={[missingProjectionEntry]}
      />,
    )

    fireEvent.click(screen.getByRole('tab', { name: /Missing Projections 1/i }))
    const missingPanel = screen.getByRole('tabpanel', { name: /Missing Projections/i })

    expect(within(missingPanel).getByText(/Knowledge And Reasoning/)).toBeInTheDocument()
    expect(within(missingPanel).getByText(/athena/)).toBeInTheDocument()
    expect(within(missingPanel).getByText(/human_library_projection_for_boardroom_consumption/)).toBeInTheDocument()
  })
})
