
import { renderHook, act, waitFor } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { useLearningLoopData } from './useLearningLoopData'
import * as loader from '../../../lib/learningLoopLoader'

describe('useLearningLoopData', () => {
  beforeEach(() => {
    vi.spyOn(loader, 'loadLearningLoopData').mockResolvedValue({
      current_cycle: 1,
      last_update: '2026-06-07T21:40:06.101889200+00:00',
      deltas_processed: 2,
      proposals_made: 2,
      gated_proposals: 0,
      status: 'active',
      blockers: [],
      recent_deltas: [],
      proposals: []
    })
    
    vi.spyOn(loader, 'getRecentDeltas').mockResolvedValue([])
    vi.spyOn(loader, 'getProposals').mockResolvedValue([])
    vi.spyOn(loader, 'getBlockers').mockResolvedValue([])
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('should initialize with loading state', () => {
    const { result } = renderHook(() => useLearningLoopData())
    expect(result.current.loading).toBe(true)
  })

  it('should load data successfully', async () => {
    const { result } = renderHook(() => useLearningLoopData())
    
    await waitFor(() => expect(result.current.loading).toBe(false))
    
    expect(result.current.loading).toBe(false)
    expect(result.current.error).toBe(null)
    expect(result.current.loopState).toBeDefined()
    expect(result.current.deltas).toEqual([])
    expect(result.current.proposals).toEqual([])
    expect(result.current.blockers).toEqual([])
  })

  it('should handle errors', async () => {
    vi.spyOn(loader, 'loadLearningLoopData').mockRejectedValue(new Error('Failed to load'))
    
    const { result } = renderHook(() => useLearningLoopData())
    
    await waitFor(() => expect(result.current.loading).toBe(false))
    
    expect(result.current.loading).toBe(false)
    expect(result.current.error).toBe('Failed to load')
  })

  it('should refresh data when refreshData is called', async () => {
    const { result } = renderHook(() => useLearningLoopData())
    
    await waitFor(() => expect(result.current.loading).toBe(false))
    
    const initialLastUpdated = result.current.lastUpdated
    
    // Mock new data
    vi.spyOn(loader, 'loadLearningLoopData').mockResolvedValue({
      current_cycle: 2,
      last_update: '2026-06-07T21:40:06.101889200+00:00',
      deltas_processed: 2,
      proposals_made: 2,
      gated_proposals: 0,
      status: 'active',
      blockers: [],
      recent_deltas: [],
      proposals: [],
    })
    
    act(() => {
      result.current.refreshData()
    })
    
    await waitFor(() => expect(result.current.loopState?.current_cycle).toBe(2))
    
    expect(result.current.loopState?.current_cycle).toBe(2)
    expect(result.current.lastUpdated).not.toBe(initialLastUpdated)
  })
})
