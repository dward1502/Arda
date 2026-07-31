import { describe, expect, it } from 'vitest'
import type { ArdaSourceProvenance } from '../../lib/ardaProvenance'
import { adaptBoardroomHudSource } from './boardroomHudSourceAdapters'

function provenance(overrides: Partial<ArdaSourceProvenance>): ArdaSourceProvenance {
  return {
    domainId: 'planning:core/state/queue_summary.json',
    label: 'Queue Summary',
    sourcePaths: ['core/state/queue_summary.json'],
    generatedAtUtc: '2026-07-30T20:00:00Z',
    observedAtUtc: '2026-07-30T20:01:00Z',
    state: 'fresh',
    sourceKind: 'snapshot',
    ...overrides,
  }
}

describe('adaptBoardroomHudSource', () => {
  it('matches live provenance by source path rather than generated domain ID', () => {
    const source = adaptBoardroomHudSource([
      provenance({ domainId: 'planning:core/state/queue_summary.json' }),
    ], 'queue')

    expect(source).toEqual({
      sourceId: 'planning:core/state/queue_summary.json',
      sourceIds: ['planning:core/state/queue_summary.json'],
      sourcePaths: ['core/state/queue_summary.json'],
      observedAtUtc: '2026-07-30T20:01:00Z',
      freshness: 'fresh',
    })
  })

  it('combines bounded source paths and reports the worst matched freshness', () => {
    const sources = Array.from({ length: 10 }, (_, index) => provenance({
      domainId: `knowledge:${index}`,
      sourcePaths: [`data/athena/digest-${index}.jsonl`],
      observedAtUtc: `2026-07-30T20:${String(index).padStart(2, '0')}:00Z`,
      state: index === 4 ? 'stale' : 'fresh',
    }))
    const source = adaptBoardroomHudSource(sources, 'knowledge')

    expect(source.sourcePaths).toHaveLength(8)
    expect(source.sourceIds).toHaveLength(8)
    expect(source.sourceId).toBe('knowledge:9')
    expect(source.sourceIds?.[0]).toBe('knowledge:9')
    expect(source.observedAtUtc).toBe('2026-07-30T20:09:00Z')
    expect(source.freshness).toBe('stale')
  })

  it('fails visibly with canonical expected paths when the source family is absent', () => {
    const source = adaptBoardroomHudSource([], 'routing')

    expect(source.sourceId).toBe('routing')
    expect(source.sourceIds).toEqual(['routing'])
    expect(source.sourcePaths).toContain('core/state/manwe_router.json')
    expect(source.observedAtUtc).toBeNull()
    expect(source.freshness).toBe('missing')
  })

  it('fails closed when a matched source is blocked', () => {
    const source = adaptBoardroomHudSource([
      provenance({ state: 'fresh' }),
      provenance({
        domainId: 'planning:core/projects/tasks/queue.jsonl',
        sourcePaths: ['core/projects/tasks/queue.jsonl'],
        state: 'blocked',
      }),
    ], 'queue')

    expect(source.sourceIds).toEqual([
      'planning:core/state/queue_summary.json',
      'planning:core/projects/tasks/queue.jsonl',
    ])
    expect(source.freshness).toBe('blocked')
  })

  it('preserves unknown provenance produced by malformed or unreadable sources', () => {
    const source = adaptBoardroomHudSource([
      provenance({ state: 'unknown', observedAtUtc: null, generatedAtUtc: null }),
    ], 'queue')

    expect(source.observedAtUtc).toBeNull()
    expect(source.freshness).toBe('unknown')
  })
})
