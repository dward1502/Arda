
import LearningLoopSurface from './LearningLoopSurface'
import { useLearningLoopData } from '../hooks/useLearningLoopData'

interface LearningLoopSurfaceWrapperProps {
  tag?: string
}

export const LearningLoopSurfaceWrapper: React.FC<LearningLoopSurfaceWrapperProps> = ({ tag }) => {
  const { loopState, deltas, proposals, blockers, loading, error } = useLearningLoopData()

  if (loading) {
    return <div>Loading Learning Loop data...</div>
  }

  if (error) {
    return <div>Error loading Learning Loop: {error}</div>
  }

  if (!loopState) {
    return <div>No Learning Loop data available</div>
  }

  // Merge blockers into loopState
  const loopStateWithBlockers = {
    ...loopState,
    blockers: blockers || []
  }

  return (
    <LearningLoopSurface 
      loopState={loopStateWithBlockers}
      knowledgeDeltas={deltas || []}
      proposals={proposals || []}
    />
  )
}

export default LearningLoopSurfaceWrapper
