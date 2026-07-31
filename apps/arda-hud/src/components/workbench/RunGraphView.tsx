import type { RunGraph, RunNode } from '../../lib/workbench'

interface RunGraphViewProps {
  graph: RunGraph | null
  onSelectNode?: (node: RunNode) => void
}

export default function RunGraphView({ graph, onSelectNode }: RunGraphViewProps) {
  if (!graph) return <section className="workbench-panel" aria-labelledby="run-graph-title"><h3 id="run-graph-title">Run graph</h3><p>No run planned. Capture an objective to prepare the graph.</p></section>
  const blocked = graph.nodes.filter((node) => node.state === 'blocked' || node.kind === 'approval' && node.state !== 'succeeded')
  return (
    <section className="workbench-panel" aria-labelledby="run-graph-title">
      <header><h3 id="run-graph-title">Run graph</h3><span>{blocked.length} blocked or gated</span></header>
      <ol className="workbench-graph" aria-label="Run sequence">
        {graph.nodes.map((node) => (
          <li key={node.id}>
            <button type="button" className={`workbench-node workbench-node--${node.state}`} onClick={() => onSelectNode?.(node)} aria-label={`${node.kind}: ${node.state}`}>
              <strong>{node.kind}</strong><span>{node.state}</span><small>{node.authority.replace(/_/g, ' ')}</small>
            </button>
          </li>
        ))}
      </ol>
    </section>
  )
}
