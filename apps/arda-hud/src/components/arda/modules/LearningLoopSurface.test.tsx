
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import LearningLoopSurface from './LearningLoopSurface'

const mockLoopState = {
  current_cycle: 1,
  last_update: "2026-06-07T21:40:06.101889200+00:00",
  deltas_processed: 2,
  proposals_made: 2,
  gated_proposals: 0,
  status: "active",
  blockers: [],
}

const mockDeltas = [
  {
    id: "delta_1",
    source: "data/athena/knowledge_deltas.jsonl",
    confidence: 0.9,
    uncertainty: 0.1,
    content: "System performance metrics show 15% improvement in processing speed",
    timestamp: "2026-06-07T21:40:06.101889200+00:00"
  },
  {
    id: "delta_2",
    source: "data/athena/knowledge_deltas.jsonl",
    confidence: 0.7,
    uncertainty: 0.2,
    content: "User feedback indicates potential issues with the new UI",
    timestamp: "2026-06-07T21:40:06.101889200+00:00"
  }
]

const mockProposals = [
  {
    id: "prop_1",
    task_id: "tsk_20260607_001",
    title: "Improve processing speed",
    description: "Implement optimizations based on performance metrics",
    priority: "high",
    risk_level: "low",
    confidence: 0.9,
    proposed_at: "2026-06-07T21:40:06.101889200+00:00",
    source_delta_id: "delta_1"
  },
  {
    id: "prop_2", 
    task_id: "tsk_20260607_002",
    title: "Review UI feedback",
    description: "Analyze user feedback and implement improvements",
    priority: "medium",
    risk_level: "medium",
    confidence: 0.7,
    proposed_at: "2026-06-07T21:40:06.101889200+00:00",
    source_delta_id: "delta_2"
  }
]

describe('LearningLoopSurface', () => {
  it('renders loop status correctly', () => {
    render(
      <LearningLoopSurface 
        loopState={mockLoopState}
        knowledgeDeltas={mockDeltas}
        proposals={mockProposals}
      />
    )
    
    expect(screen.getByText('Learning Loop v1 Status')).toBeInTheDocument()
    expect(screen.getByText('active')).toBeInTheDocument()
  })

  it('renders metrics correctly', () => {
    render(
      <LearningLoopSurface 
        loopState={mockLoopState}
        knowledgeDeltas={mockDeltas}
        proposals={mockProposals}
      />
    )
    
    expect(screen.getByText('Current Cycle')).toBeInTheDocument()
    expect(screen.getByText('Deltas Processed')).toBeInTheDocument()
    expect(screen.getByText('Proposals Made')).toBeInTheDocument()
    expect(screen.getByText('Gated Proposals')).toBeInTheDocument()
  })

  it('renders recent deltas', () => {
    render(
      <LearningLoopSurface 
        loopState={mockLoopState}
        knowledgeDeltas={mockDeltas}
        proposals={mockProposals}
      />
    )
    
    expect(screen.getByText('Recent Deltas')).toBeInTheDocument()
    expect(screen.getByText('System performance metrics show 15% improvement in processing speed')).toBeInTheDocument()
    expect(screen.getByText('User feedback indicates potential issues with the new UI')).toBeInTheDocument()
  })

  it('renders proposals', () => {
    render(
      <LearningLoopSurface 
        loopState={mockLoopState}
        knowledgeDeltas={mockDeltas}
        proposals={mockProposals}
      />
    )
    
    expect(screen.getByText('Proposals')).toBeInTheDocument()
    expect(screen.getByText('Improve processing speed')).toBeInTheDocument()
    expect(screen.getByText('Review UI feedback')).toBeInTheDocument()
  })

  it('renders next action correctly', () => {
    render(
      <LearningLoopSurface 
        loopState={mockLoopState}
        knowledgeDeltas={mockDeltas}
        proposals={mockProposals}
      />
    )
    
    expect(screen.getByText('Next Action')).toBeInTheDocument()
    expect(screen.getByText('Execute active proposals')).toBeInTheDocument()
  })
})
