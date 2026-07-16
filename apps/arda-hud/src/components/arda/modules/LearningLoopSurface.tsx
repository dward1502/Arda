
import React from 'react'
import { Activity, BookOpenText, Briefcase, FolderKanban, Settings, Shield } from 'lucide-react'

interface KnowledgeDelta {
  id: string
  source: string
  confidence: number
  uncertainty: number
  content: string
  timestamp: string
}

interface TaskProposal {
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

interface LearningLoopState {
  current_cycle: number
  last_update: string
  deltas_processed: number
  proposals_made: number
  gated_proposals: number
  status: 'active' | 'blocked' | 'completed'
  blockers: string[]
}

interface ARDAOperatorSurfaceProps {
  loopState: LearningLoopState
  knowledgeDeltas: KnowledgeDelta[]
  proposals: TaskProposal[]
}

const ARDAOperatorSurface: React.FC<ARDAOperatorSurfaceProps> = ({ loopState, knowledgeDeltas, proposals }) => {
  // Calculate metrics
  const totalDeltas = knowledgeDeltas.length
  const totalProposals = proposals.length
  const gatedProposals = proposals.filter(p => p.risk_level === 'high').length
  const activeProposals = proposals.filter(p => p.risk_level !== 'high').length

  // Determine next action
  let nextAction = 'Wait for new knowledge deltas'
  if (loopState.blockers.length > 0) {
    nextAction = 'Resolve blockers'
  } else if (gatedProposals > 0) {
    nextAction = 'Review gated proposals for HADES approval'
  } else if (activeProposals > 0) {
    nextAction = 'Execute active proposals'
  }

  return (
    <div className="arda-learning-loop-surface">
      <div className="operator-header">
        <h2>Learning Loop v1 Status</h2>
        <span className={`status-badge ${loopState.status}`}>{loopState.status}</span>
      </div>

      <div className="metrics-grid">
        <div className="metric-card">
          <span className="metric-label">Current Cycle</span>
          <span className="metric-value">{loopState.current_cycle}</span>
        </div>
        <div className="metric-card">
          <span className="metric-label">Deltas Processed</span>
          <span className="metric-value">{totalDeltas}</span>
        </div>
        <div className="metric-card">
          <span className="metric-label">Proposals Made</span>
          <span className="metric-value">{totalProposals - gatedProposals}</span>
        </div>
        <div className="metric-card">
          <span className="metric-label">Gated Proposals</span>
          <span className="metric-value">{gatedProposals}</span>
        </div>
      </div>

      {loopState.blockers.length > 0 && (
        <div className="blockers-section">
          <h3>Blockers</h3>
          <ul>
            {loopState.blockers.map((blocker, index) => (
              <li key={index}>{blocker}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="deltas-section">
        <h3>Recent Deltas</h3>
        {knowledgeDeltas.map(delta => (
          <div key={delta.id} className="delta-card">
            <div className="delta-header">
              <span className="delta-id">ID: {delta.id}</span>
              <span className="delta-confidence">Confidence: {delta.confidence}</span>
            </div>
            <p className="delta-content">{delta.content}</p>
            <span className="delta-source">{delta.source}</span>
          </div>
        ))}
      </div>

      <div className="proposals-section">
        <h3>Proposals</h3>
        {proposals.map(proposal => (
          <div key={proposal.id} className="proposal-card">
            <div className="proposal-header">
              <h4>{proposal.title}</h4>
              <span className={`proposal-risk ${proposal.risk_level}`}>{proposal.risk_level}</span>
            </div>
            <p>{proposal.description}</p>
            <div className="proposal-meta">
              <span>Priority: {proposal.priority}</span>
              <span>Confidence: {proposal.confidence}</span>
              <span>Proposed: {new Date(proposal.proposed_at).toLocaleString()}</span>
            </div>
          </div>
        ))}
      </div>

      <div className="next-action">
        <h3>Next Action</h3>
        <p>{nextAction}</p>
      </div>
    </div>
  )
}

export default ARDAOperatorSurface
