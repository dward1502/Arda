import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import RunGraphView from './RunGraphView'
import type { RunGraph } from '../../lib/workbench'

const graph: RunGraph = { schema_version: 'arda.run-graph.v1', run_id: 'run-1', objective_id: 'objective-1', nodes: [{ id: 'approval', kind: 'approval', state: 'blocked', authority: 'human_approval', budget: { max_joules: 1, max_cost_usd: 0 }, retry: { max_attempts: 1 }, timeout_ms: 1000, idempotency_key: 'node-1', input_digest: null, output_digest: null, parent_receipts: [], checkpoint: { sequence: 0, recovery_token: null, checkpoint_digest: null } }], edges: [], provenance: { project_contract_digest: 'digest', created_by: 'test', parent_receipts: [] } }

describe('RunGraphView', () => {
  it('exposes graph state and keyboard-operable nodes', () => {
    const onSelect = vi.fn()
    render(<RunGraphView graph={graph} onSelectNode={onSelect} />)
    const node = screen.getByRole('button', { name: 'approval: blocked' })
    node.focus(); fireEvent.keyDown(node, { key: 'Enter' }); fireEvent.click(node)
    expect(document.activeElement).toBe(node); expect(onSelect).toHaveBeenCalledWith(graph.nodes[0])
    expect(screen.getByText('1 blocked or gated')).toBeTruthy()
  })
})
