import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ResearchModule from './ResearchModule'

const invokeMock = vi.hoisted(() => vi.fn())

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }))

const emptyProjection = {
  schemaVersion: 'arda.hud.research-projection.v1',
  state: 'healthy',
  sourceRevision: 'research-revision-1',
  sourceTimeUtc: '2026-08-11T16:00:00Z',
  questions: [],
  watchlists: [],
  briefs: [],
  failures: [],
  recoveryAction: null,
}

beforeEach(() => {
  invokeMock.mockReset()
  invokeMock.mockResolvedValue(emptyProjection)
  Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {}, configurable: true })
})

describe('ResearchModule', () => {
  it('reads one aggregate Rust projection and does not expose browser authority fields', async () => {
    render(<ResearchModule />)
    expect(await screen.findByRole('heading', { name: 'Compose explicit question' })).toBeTruthy()
    expect(screen.getByRole('textbox', { name: 'Question' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Create bounded question' })).toBeTruthy()
    expect(screen.getByRole('heading', { name: 'Compose watchlist' })).toBeTruthy()
    expect(screen.getByRole('textbox', { name: 'Approval reference' })).toBeTruthy()
    expect(screen.queryByRole('textbox', { name: 'Owner' })).toBeNull()
    expect(screen.queryByRole('textbox', { name: 'Proposal ID' })).toBeNull()
    expect(screen.queryByRole('textbox', { name: 'Approval ID' })).toBeNull()
    expect(invokeMock).toHaveBeenCalledTimes(1)
    expect(invokeMock).toHaveBeenCalledWith('get_research_projection', undefined)
  })

  it('discloses bounded Rumil completeness and projection freshness', async () => {
    invokeMock.mockResolvedValue({
      ...emptyProjection,
      state: 'stale',
      sourceRevision: 'research-revision-stale',
      recoveryAction: 'Refresh Research after restoring the brief owner.',
      briefs: [{
        schema_version: 'arda.workbench.research-brief.v1', brief_id: 'brief-rumil', question: 'Audit evidence', citations: [], stale: true,
        rumil_evidence: {
          audit_id: 'audit-1', project_id: 'project-1', packet_reference: 'data/rumil/audit-1.json', packet_sha256: 'abc123',
          completeness: 'partial', evidence_classes: ['tool_backed', 'partial', 'unavailable'], stale_baseline: true,
          rejected_providers: ['rumil.cargo_audit.v1'], missing_evidence: ['dependency_security'], evaluation_status: 'review_required',
          degraded_reasons: ['coverage is partial'], authority: 'advisory_read_only', execution_authorized: false,
        },
      }],
    })

    render(<ResearchModule />)
    expect(await screen.findByRole('region', { name: 'Rúmil audit evidence' })).toBeTruthy()
    expect(screen.getByText(/Research workspace stale at revision research-revision-stale/)).toBeTruthy()
    expect(screen.getByText('partial')).toBeTruthy()
    expect(screen.getByText('rumil.cargo_audit.v1')).toBeTruthy()
    expect(screen.getByText('dependency_security')).toBeTruthy()
    expect(screen.getByText('Advisory only — execution disabled')).toBeTruthy()
  })
})