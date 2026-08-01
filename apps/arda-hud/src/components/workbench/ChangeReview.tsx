import type { ChangeRecord, ProviderReceiptRecord, RunNode, TestRecord, WorkbenchEvent } from '../../lib/workbench'

interface ChangeReviewProps { changes: ChangeRecord[]; tests: TestRecord[]; providerReceipt?: ProviderReceiptRecord | null; selectedNode?: RunNode | null; events?: WorkbenchEvent[]; onComplete?: () => void; onExecuteProvider?: () => void; busy?: boolean }

export default function ChangeReview({ changes, tests, providerReceipt, selectedNode, events = [], onComplete, onExecuteProvider, busy = false }: ChangeReviewProps) {
  const nodeEvents = selectedNode ? events.filter((event) => event.node_id === selectedNode.id) : []
  return (
    <section className="workbench-panel" aria-labelledby="change-review-title">
      <header><h3 id="change-review-title">Objective review</h3><span>{changes.length} files / {tests.length} checks</span></header>
      {selectedNode ? <dl className="workbench-receipt" aria-label="Selected run node review">
        <div><dt>Node</dt><dd>{selectedNode.id} · {selectedNode.kind}</dd></div>
        <div><dt>State</dt><dd>{selectedNode.state} · {selectedNode.authority.replace(/_/g, ' ')}</dd></div>
        <div><dt>Input</dt><dd>{selectedNode.input_digest ?? 'not recorded'}</dd></div>
        <div><dt>Output</dt><dd>{selectedNode.output_digest ?? 'not recorded'}</dd></div>
        <div><dt>Receipts</dt><dd>{selectedNode.parent_receipts.join(', ') || 'none'} · {nodeEvents.length} live events</dd></div>
      </dl> : <p>Select a run-graph node to inspect its review boundary.</p>}
      {selectedNode?.kind === 'execute' && selectedNode.state !== 'succeeded' ? <button type="button" disabled={busy} onClick={onExecuteProvider}>Execute approved node with live provider</button> : null}
      {selectedNode && ['verify', 'review', 'close'].includes(selectedNode.kind) && selectedNode.state !== 'succeeded' ? <button type="button" disabled={busy} onClick={onComplete}>Record {selectedNode.kind} receipt</button> : null}
      <div className="workbench-review-grid">
        <div><h4>Changed</h4>{changes.length === 0 ? <p>No changes recorded.</p> : <ul className="workbench-list">{changes.map((change) => <li key={change.path}><details><summary><strong>{change.path}</strong> <span>{change.status} +{change.additions} −{change.deletions}</span></summary>{change.diff ? <pre tabIndex={0}>{change.diff}</pre> : <p>Diff receipt is not available.</p>}</details></li>)}</ul>}</div>
        <div><h4>Tests</h4>{tests.length === 0 ? <p>No tests have run.</p> : <ul className="workbench-list">{tests.map((test) => <li key={test.name}><strong>{test.name}</strong><span>{test.status}{test.durationMs == null && test.duration_ms == null ? '' : ` / ${test.durationMs ?? test.duration_ms} ms`}</span>{test.details ? <small>{test.details}</small> : null}</li>)}</ul>}</div>
      </div>
      <div className="workbench-provider-receipt" aria-label="Provider receipt">
        {providerReceipt ? <><strong>{providerReceipt.provider} / {providerReceipt.model}</strong><span>{providerReceipt.adapter} · {providerReceipt.receipt_digest}</span><small>{providerReceipt.summary}</small></> : <p>No provider receipt recorded.</p>}
      </div>
    </section>
  )
}
