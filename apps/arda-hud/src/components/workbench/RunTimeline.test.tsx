import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import RunTimeline from './RunTimeline'
import type { RunGraph } from '../../lib/workbench'

const graph: RunGraph = { schema_version: 'arda.run-graph.v1', run_id: 'run-1', objective_id: 'objective-1', nodes: [{ id: 'execute', kind: 'execute', state: 'running', authority: 'execute_with_approval', budget: { max_joules: 100, max_cost_usd: 1.25 }, retry: { max_attempts: 1 }, timeout_ms: 1000, idempotency_key: 'key', input_digest: null, output_digest: null, parent_receipts: [], checkpoint: { sequence: 2, recovery_token: 'resume-2', checkpoint_digest: 'sha256:2' } }], edges: [], provenance: { project_contract_digest: 'digest', created_by: 'test', parent_receipts: [] } }

describe('RunTimeline', () => {
  it('shows costs, events, resume path, and reduced-motion posture', () => {
    Object.defineProperty(window, 'matchMedia', { configurable: true, value: vi.fn(() => ({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() })) })
    const { container } = render(<RunTimeline graph={graph} events={[{ sequence: 1, node_id: 'execute', event: { type: 'node_transition', state: 'running' } }]} />)
    expect(screen.getByText('$1.25 maximum')).toBeTruthy(); expect(screen.getByText('resume-2')).toBeTruthy()
    expect(screen.getByText('node transition')).toBeTruthy(); expect(container.querySelector('.workbench-timeline--reduced-motion')).toBeTruthy()
  })
})
