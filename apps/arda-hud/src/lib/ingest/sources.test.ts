// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { normalizeComponentType } from './sources'

describe('ingest data sources', () => {
  it('normalizes optional component types without unsafe casts', () => {
    expect(normalizeComponentType('metric')).toBe('metric')
    expect(normalizeComponentType('status-matrix')).toBe('status-matrix')
    expect(normalizeComponentType(undefined)).toBe('auto')
    expect(normalizeComponentType('typo-component')).toBe('auto')
  })
})
