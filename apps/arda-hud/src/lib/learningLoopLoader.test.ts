
import { describe, it, expect, vi } from 'vitest'
import { loadLearningLoopData, getLearningLoopStatus, getLearningLoopMetrics, getRecentDeltas, getProposals, getBlockers } from './learningLoopLoader'

vi.mock('./weathertop', () => ({
  readFile: vi.fn(async () => ({
    success: true,
    content: JSON.stringify({
      current_cycle: 1,
      last_update: '2026-06-12T00:00:00Z',
      deltas_processed: 2,
      proposals_made: 1,
      gated_proposals: 0,
      status: 'active',
      blockers: [],
      recent_deltas: [
        {
          id: 'delta-1',
          source: 'test',
          confidence: 0.8,
          uncertainty: 0.2,
          content: 'Test delta',
          timestamp: '2026-06-12T00:00:00Z',
        },
      ],
      proposals: [
        {
          id: 'proposal-1',
          task_id: 'tsk_test',
          title: 'Test proposal',
          description: 'Test proposal',
          priority: 'medium',
          risk_level: 'low',
          confidence: 0.8,
          proposed_at: '2026-06-12T00:00:00Z',
          source_delta_id: 'delta-1',
        },
      ],
    }),
    error: null,
    path: 'core/state/learning_loop_v1.json',
  })),
}))

describe('learningLoopLoader', () => {
  it('should load learning loop data', async () => {
    const data = await loadLearningLoopData()
    expect(data).toBeDefined()
    expect(data.current_cycle).toBeGreaterThan(0)
    expect(data.status).toBeOneOf(['active', 'blocked', 'completed'])
  })

  it('should get learning loop status', async () => {
    const status = await getLearningLoopStatus()
    expect(status).toBeOneOf(['active', 'blocked', 'completed', 'error'])
  })

  it('should get learning loop metrics', async () => {
    const metrics = await getLearningLoopMetrics()
    expect(metrics).toBeDefined()
    expect(metrics.current_cycle).toBeGreaterThanOrEqual(0)
    expect(metrics.deltas_processed).toBeGreaterThanOrEqual(0)
    expect(metrics.proposals_made).toBeGreaterThanOrEqual(0)
    expect(metrics.gated_proposals).toBeGreaterThanOrEqual(0)
  })

  it('should get recent deltas', async () => {
    const deltas = await getRecentDeltas()
    expect(Array.isArray(deltas)).toBe(true)
    if (deltas.length > 0) {
      const delta = deltas[0]
      expect(delta.id).toBeDefined()
      expect(delta.content).toBeDefined()
      expect(delta.confidence).toBeGreaterThanOrEqual(0)
    }
  })

  it('should get proposals', async () => {
    const proposals = await getProposals()
    expect(Array.isArray(proposals)).toBe(true)
    if (proposals.length > 0) {
      const proposal = proposals[0]
      expect(proposal.id).toBeDefined()
      expect(proposal.title).toBeDefined()
      expect(proposal.priority).toBeOneOf(['high', 'medium', 'low'])
    }
  })

  it('should get blockers', async () => {
    const blockers = await getBlockers()
    expect(Array.isArray(blockers)).toBe(true)
  })
})
