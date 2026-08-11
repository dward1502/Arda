import { describe, expect, it } from 'vitest'
import { buildResearchUrl, formatCadence, newQuestionDraft, projectResearchState } from './research'

describe('research projection', () => {
  it('builds bounded drafts with public-only source policy', () => {
    const draft = newQuestionDraft()
    expect(draft.schema_version).toBe('arda.warden.watchlist.v1')
    expect(draft.source_policy.allow_private_targets).toBe(false)
    expect(draft.budgets.max_fetch_bytes).toBeGreaterThan(0)
  })

  it('keeps lifecycle states distinct', () => {
    expect(projectResearchState('preview').description).toMatch(/not evidence/)
    expect(projectResearchState('evaluated').label).toBe('Evaluation')
    expect(projectResearchState('approved').label).toBe('Approved knowledge')
    expect(projectResearchState('proposal').tone).toBe('warning')
    expect(projectResearchState('rejected').tone).toBe('danger')
  })

  it('formats manual and interval cadence without pretending scheduling is local', () => {
    expect(formatCadence({ kind: 'manual' })).toBe('Manual')
    expect(formatCadence({ kind: 'interval', every_seconds: 7200 })).toBe('Every 2h')
  })

  it('uses the configured harness URL without duplicating slashes', () => {
    expect(buildResearchUrl('/v1/research/briefs', 'http://127.0.0.1:7878/')).toBe('http://127.0.0.1:7878/v1/research/briefs')
  })
})
