import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ResearchModule from './ResearchModule'

const fetchMock = vi.fn()

beforeEach(() => {
  fetchMock.mockReset()
  fetchMock.mockImplementation((input: RequestInfo | URL) => {
    const url = String(input)
    const body = url.endsWith('/status') ? { operator_id: 'operator-0' } : url.endsWith('/questions') ? { questions: [] } : url.endsWith('/watchlists') ? { watchlists: [] } : { briefs: [] }
    return Promise.resolve(new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } }))
  })
  vi.stubGlobal('fetch', fetchMock)
})

describe('ResearchModule', () => {
  it('exposes explicit question and bounded watchlist flows before shell wiring', async () => {
    render(<ResearchModule />)
    expect(await screen.findByRole('heading', { name: 'Compose explicit question' })).toBeTruthy()
    expect(screen.getByRole('textbox', { name: 'Question' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Create bounded question' })).toBeTruthy()
    expect(screen.getByRole('heading', { name: 'Compose watchlist' })).toBeTruthy()
    expect(screen.getByText('Warden → Varda → advisory brief')).toBeTruthy()
    expect(fetchMock).toHaveBeenCalledTimes(4)
    for (const [, init] of fetchMock.mock.calls.slice(1)) {
      expect((init as RequestInit).headers).toMatchObject({ 'x-arda-operator-id': 'operator-0' })
    }
  })

  it('discloses bounded Rumil completeness and degraded evidence states', async () => {
    fetchMock.mockImplementation((input: RequestInfo | URL) => {
      const url = String(input)
      const body = url.endsWith('/status')
        ? { operator_id: 'operator-0' }
        : url.endsWith('/questions')
        ? { questions: [] }
        : url.endsWith('/watchlists')
          ? { watchlists: [] }
          : {
              briefs: [{
                schema_version: 'arda.workbench.research-brief.v1',
                brief_id: 'brief-rumil',
                question: 'Audit evidence',
                citations: [],
                stale: true,
                rumil_evidence: {
                  audit_id: 'audit-1',
                  project_id: 'project-1',
                  packet_reference: 'data/rumil/audit-1.json',
                  packet_sha256: 'abc123',
                  completeness: 'partial',
                  evidence_classes: ['tool_backed', 'partial', 'unavailable'],
                  stale_baseline: true,
                  rejected_providers: ['rumil.cargo_audit.v1'],
                  missing_evidence: ['dependency_security'],
                  evaluation_status: 'review_required',
                  degraded_reasons: ['coverage is partial'],
                  authority: 'advisory_read_only',
                  execution_authorized: false,
                },
              }],
            }
      return Promise.resolve(new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } }))
    })

    render(<ResearchModule />)
    expect(await screen.findByRole('region', { name: 'Rúmil audit evidence' })).toBeTruthy()
    expect(screen.getByText('partial')).toBeTruthy()
    expect(screen.getByText('rumil.cargo_audit.v1')).toBeTruthy()
    expect(screen.getByText('dependency_security')).toBeTruthy()
    expect(screen.getByText('Advisory only — execution disabled')).toBeTruthy()
  })

  it('renders the canonical backend brief summary and citation provenance', async () => {
    fetchMock.mockImplementation((input: RequestInfo | URL) => {
      const url = String(input)
      const body = url.endsWith('/status')
        ? { operator_id: 'operator-0' }
        : url.endsWith('/questions')
          ? { questions: [] }
          : url.endsWith('/watchlists')
            ? { watchlists: [] }
            : {
                briefs: [{
                  schema_version: 'arda.workbench.research-brief.v1',
                  brief_id: 'brief-x402',
                  question: 'How does x402 support paid APIs?',
                  summary: 'Two current canonical sources describe paid API access.',
                  authority: 'advisory_research_evidence',
                  execution_authorized: false,
                  citations: [{
                    citation_id: 'cite-x402',
                    title: 'Facilitator - x402',
                    canonical_url: 'https://docs.x402.org/core-concepts/facilitator',
                    excerpt: 'A facilitator verifies payments and submits settlements.',
                    policy_readiness: 'reference_only',
                    freshness_status: 'fresh',
                    confidence: 0.74,
                  }],
                  source_failures: [],
                  next_research_or_proposal: ['Review evidence before proposing action.'],
                }],
              }
      return Promise.resolve(new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } }))
    })

    render(<ResearchModule />)

    expect(await screen.findByText('Two current canonical sources describe paid API access.')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: /Facilitator - x402/ }))
    expect(screen.getByRole('link', { name: 'Open canonical source' }).getAttribute('href'))
      .toBe('https://docs.x402.org/core-concepts/facilitator')
    expect(screen.getByText('Advisory evidence only — execution is not authorized.')).toBeTruthy()
  })
})
