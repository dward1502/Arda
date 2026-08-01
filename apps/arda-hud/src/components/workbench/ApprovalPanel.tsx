import type { RunNode } from '../../lib/workbench'

interface ApprovalPanelProps {
  approvals: RunNode[]
  busy?: boolean
  onApprove: (nodeId: string) => void
  onReject?: (nodeId: string) => void
}

export default function ApprovalPanel({ approvals, busy = false, onApprove, onReject }: ApprovalPanelProps) {
  const pending = approvals.filter((node) => node.state !== 'succeeded' && node.state !== 'cancelled')
  return (
    <section className="workbench-panel" aria-labelledby="approval-panel-title">
      <header><h3 id="approval-panel-title">Approvals</h3><span>{pending.length} need attention</span></header>
      {pending.length === 0 ? <p>No approvals are waiting.</p> : (
        <ul className="workbench-list">
          {pending.map((node) => (
            <li key={node.id}>
              <div><strong>{node.id}</strong><span>{node.authority.replace(/_/g, ' ')}</span><small>Budget ${node.budget.max_cost_usd.toFixed(2)} / {node.budget.max_joules} J</small></div>
              <div className="workbench-actions">
                <button type="button" disabled={busy} onClick={() => onApprove(node.id)}>Approve</button>
                {onReject ? <button type="button" disabled={busy} onClick={() => onReject(node.id)}>Reject</button> : null}
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
