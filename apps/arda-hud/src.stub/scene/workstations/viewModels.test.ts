// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  createEmptyDecisionViewModel,
  createEmptyEvidenceViewModel,
  createEmptyFleetViewModel,
  createEmptyKnowledgeViewModel,
  createEmptySettingsViewModel,
  createEmptyWorkViewModel,
  isSafeActionDescriptor,
} from './viewModels'

describe('workstation view models', () => {
  it('can represent each role as a safe empty/fallback model', () => {
    const models = [
      createEmptyFleetViewModel(),
      createEmptyWorkViewModel(),
      createEmptyDecisionViewModel(),
      createEmptyKnowledgeViewModel(),
      createEmptyEvidenceViewModel(),
      createEmptySettingsViewModel(),
    ]

    expect(models.map((model) => model.roleId)).toEqual([
      'fleet',
      'work',
      'decisions',
      'knowledge',
      'evidence',
      'settings',
    ])

    for (const model of models) {
      expect(model.status).toBe('empty')
      expect(model.summary.length).toBeGreaterThan(0)
      expect(model.metrics).toEqual([])
      expect(model.sources.every((source) => source.freshness.status === 'missing')).toBe(true)
      expect('raw' in model).toBe(false)
    }
  })

  it('requires actions to declare their safety class', () => {
    expect(isSafeActionDescriptor({ id: 'refresh', label: 'Refresh', safety: 'read_only' })).toBe(true)
    expect(isSafeActionDescriptor({ id: 'dry-run', label: 'Dry Run', safety: 'dry_run' })).toBe(true)
    expect(isSafeActionDescriptor({ id: 'apply', label: 'Apply', safety: 'governed_mutation' })).toBe(true)
    expect(isSafeActionDescriptor({ id: 'unsafe', label: 'Unsafe' })).toBe(false)
  })
})
