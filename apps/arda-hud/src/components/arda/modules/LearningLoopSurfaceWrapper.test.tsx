
import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import LearningLoopSurfaceWrapper from './LearningLoopSurfaceWrapper'
import * as loader from '../../../lib/learningLoopLoader'

describe('LearningLoopSurfaceWrapper Integration', () => {
  beforeEach(() => {
    vi.spyOn(loader, 'loadLearningLoopData').mockReturnValue({
      current_cycle: 1,
      last_update: '2026-06-07T21:40:06.101889200+00:00',
      deltas_processed: 2,
      proposals_made: 2,
      gated_proposals: 0,
      status: 'active',
      blockers: [],
      recent_deltas: [
        {
          id: 'delta_1',
          source: 'data/athena/knowledge_deltas.jsonl',
          confidence: 0.9,
          uncertainty: 0.1,
          content: 'System performance metrics show 15% improvement in processing speed',
          timestamp: '2026-06-07T21:40:06.101889200+00:00'
        }
      ],
      proposals: [
        {
          id: 'prop_1',
          task_id: 'tsk_20260607_001',
          title: 'Improve processing speed',
          description: 'Implement optimizations based on performance metrics',
          priority: 'high',
          risk_level: 'low',
          confidence: 0.9,
          proposed_at: '2026-06-07T21:40:06.101889200+00:00',
          source_delta_id: 'delta_1'
        }
      ]
    })
    
    vi.spyOn(loader, 'getRecentDeltas').mockReturnValue([
      {
        id: 'delta_1',
        source: 'data/athena/knowledge_deltas.jsonl',
        confidence: 0.9,
        uncertainty: 0.1,
        content: 'System performance metrics show 15% improvement in processing speed',
        timestamp: '2026-06-07T21:40:06.101889200+00:00'
      }
    ])
    
    vi.spyOn(loader, 'getProposals').mockReturnValue([
      {
        id: 'prop_1',
        task_id: 'tsk_20260607_001',
        title: 'Improve processing speed',
        description: 'Implement optimizations based on performance metrics',
        priority: 'high',
        risk_level: 'low',
        confidence: 0.9,
        proposed_at: '2026-06-07T21:40:06.101889200+00:00',
        source_delta_id: 'delta_1'
      }
    ])
    
    vi.spyOn(loader, 'getBlockers').mockReturnValue([])
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('should render loading state initially', () => {
    render(<LearningLoopSurfaceWrapper />)
    expect(screen.getByText('Loading Learning Loop data...')).toBeInTheDocument()
  })

  it('should render error state when data loading fails', async () => {
    vi.spyOn(loader, 'loadLearningLoopData').mockImplementation(() => {
      throw new Error('Failed to load')
    })
    
    render(<LearningLoopSurfaceWrapper />)
    
    await waitFor(() => {
      expect(screen.getByText('Error loading Learning Loop: Failed to load')).toBeInTheDocument()
    })
  })

  it('should render the Learning Loop surface with data', async () => {
    render(<LearningLoopSurfaceWrapper />)
    
    await waitFor(() => {
      expect(screen.getByText('Learning Loop v1 Status')).toBeInTheDocument()
      expect(screen.getByText('active')).toBeInTheDocument()
      expect(screen.getByText('Current Cycle')).toBeInTheDocument()
      expect(screen.getByText('Deltas Processed')).toBeInTheDocument()
      expect(screen.getByText('Proposals Made')).toBeInTheDocument()
      expect(screen.getByText('Gated Proposals')).toBeInTheDocument()
      expect(screen.getByText('Recent Deltas')).toBeInTheDocument()
      expect(screen.getByText('System performance metrics show 15% improvement in processing speed')).toBeInTheDocument()
      expect(screen.getByText('Proposals')).toBeInTheDocument()
      expect(screen.getByText('Improve processing speed')).toBeInTheDocument()
      expect(screen.getByText('Next Action')).toBeInTheDocument()
    })
  })

  it('should render blockers when present', async () => {
    vi.spyOn(loader, 'loadLearningLoopData').mockReturnValue({
      ...loader.loadLearningLoopData(),
      blockers: ['Blocker 1', 'Blocker 2']
    })
    
    vi.spyOn(loader, 'getBlockers').mockReturnValue(['Blocker 1', 'Blocker 2'])
    
    render(<LearningLoopSurfaceWrapper />)
    
    await waitFor(() => {
      expect(screen.getByText('Blockers')).toBeInTheDocument()
      expect(screen.getByText('Blocker 1')).toBeInTheDocument()
      expect(screen.getByText('Blocker 2')).toBeInTheDocument()
    })
  })

  it('should show "Resolve blockers" as next action when blockers are present', async () => {
    vi.spyOn(loader, 'loadLearningLoopData').mockReturnValue({
      ...loader.loadLearningLoopData(),
      blockers: ['Blocker 1']
    })
    
    vi.spyOn(loader, 'getBlockers').mockReturnValue(['Blocker 1'])
    
    render(<LearningLoopSurfaceWrapper />)
    
    await waitFor(() => {
      expect(screen.getByText('Resolve blockers')).toBeInTheDocument()
    })
  })
})
