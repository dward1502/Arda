import { describe, expect, it } from 'vitest'
import { formatCadence, newQuestionDraft, projectResearchState } from './research'

describe('research projection', () => {
  it('builds bounded drafts with public-only source policy', () => {
    const draft = newQuestionDraft()
    expect('schema_version' in draft).toBe(false)
    expect('question_id' in draft).toBe(false)
    expect('owner' in draft).toBe(false)
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
})
