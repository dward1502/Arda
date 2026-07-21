
import { useState, useEffect } from 'react'
import {
  getBlockers,
  getProposals,
  getRecentDeltas,
  loadLearningLoopData,
  type KnowledgeDelta,
  type LearningLoopState,
  type TaskProposal,
} from '../../../lib/learningLoopLoader'

export function useLearningLoopData() {
  const [loopState, setLoopState] = useState<LearningLoopState | null>(null)
  const [deltas, setDeltas] = useState<KnowledgeDelta[]>([])
  const [proposals, setProposals] = useState<TaskProposal[]>([])
  const [blockers, setBlockers] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [lastUpdated, setLastUpdated] = useState<string | null>(null)

  const refreshData = async () => {
    try {
      const [data, recentDeltas, proposalRows, blockerRows] = await Promise.all([
        loadLearningLoopData(),
        getRecentDeltas(),
        getProposals(),
        getBlockers(),
      ])
      setLoopState(data)
      setDeltas(recentDeltas)
      setProposals(proposalRows)
      setBlockers(blockerRows)
      setLastUpdated(new Date().toISOString())
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    refreshData()
    
    // Set up interval for periodic refresh
    const intervalId = setInterval(refreshData, 30000) // Refresh every 30 seconds
    
    return () => clearInterval(intervalId)
  }, [])

  return {
    loopState,
    deltas,
    proposals,
    blockers,
    loading,
    error,
    lastUpdated,
    refreshData, // Expose refresh function for manual refresh
  }
}
