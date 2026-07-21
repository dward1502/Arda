import { readFile } from './weathertop'
import { jsonParseFailureMessage, parseJsonResult, type JsonParseErr } from './jsonParse'

export interface KnowledgeDelta {
  id: string
  source: string
  confidence: number
  uncertainty: number
  content: string
  timestamp: string
}

export interface TaskProposal {
  id: string
  task_id: string
  title: string
  description: string
  priority: 'high' | 'medium' | 'low'
  risk_level: 'high' | 'medium' | 'low'
  confidence: number
  proposed_at: string
  source_delta_id: string
}

export interface LearningLoopState {
  current_cycle: number
  last_update: string
  deltas_processed: number
  proposals_made: number
  gated_proposals: number
  status: 'active' | 'blocked' | 'completed'
  blockers: string[]
  recent_deltas: KnowledgeDelta[]
  proposals: TaskProposal[]
}

const LEARNING_LOOP_PATH = 'core/state/learning_loop_v1.json'

function emptyLearningLoopState(): LearningLoopState {
  return {
    current_cycle: 0,
    last_update: '',
    deltas_processed: 0,
    proposals_made: 0,
    gated_proposals: 0,
    status: 'blocked',
    blockers: ['learning loop state unavailable'],
    recent_deltas: [],
    proposals: [],
  }
}

export async function loadLearningLoopData(): Promise<LearningLoopState> {
  try {
    const result = await readFile(LEARNING_LOOP_PATH)
    if (!result.success || !result.content) {
      return emptyLearningLoopState()
    }
    const parsed = parseJsonResult<LearningLoopState>(result.content)
    if (parsed.ok) return parsed.value
    throw new Error(jsonParseFailureMessage(LEARNING_LOOP_PATH, parsed as JsonParseErr))
  } catch (error) {
    console.error('Error loading learning loop data:', error)
    throw new Error('Failed to load learning loop data')
  }
}

export async function getLearningLoopStatus(): Promise<string> {
  try {
    const data = await loadLearningLoopData()
    return data.status
  } catch (error) {
    return 'error'
  }
}

export async function getLearningLoopMetrics(): Promise<{
  current_cycle: number
  deltas_processed: number
  proposals_made: number
  gated_proposals: number
}> {
  try {
    const data = await loadLearningLoopData()
    return {
      current_cycle: data.current_cycle,
      deltas_processed: data.deltas_processed,
      proposals_made: data.proposals_made,
      gated_proposals: data.gated_proposals,
    }
  } catch (error) {
    return {
      current_cycle: 0,
      deltas_processed: 0,
      proposals_made: 0,
      gated_proposals: 0,
    }
  }
}

export async function getRecentDeltas(): Promise<KnowledgeDelta[]> {
  try {
    const data = await loadLearningLoopData()
    return data.recent_deltas || []
  } catch (error) {
    return []
  }
}

export async function getProposals(): Promise<TaskProposal[]> {
  try {
    const data = await loadLearningLoopData()
    return data.proposals || []
  } catch (error) {
    return []
  }
}

export async function getBlockers(): Promise<string[]> {
  try {
    const data = await loadLearningLoopData()
    return data.blockers || []
  } catch (error) {
    return []
  }
}
