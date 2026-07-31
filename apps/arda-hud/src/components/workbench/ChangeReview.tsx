import type { ChangeRecord, TestRecord } from '../../lib/workbench'

interface ChangeReviewProps { changes: ChangeRecord[]; tests: TestRecord[] }

export default function ChangeReview({ changes, tests }: ChangeReviewProps) {
  return (
    <section className="workbench-panel" aria-labelledby="change-review-title">
      <header><h3 id="change-review-title">Changes and tests</h3><span>{changes.length} files / {tests.length} checks</span></header>
      <div className="workbench-review-grid">
        <div><h4>Changed</h4>{changes.length === 0 ? <p>No changes recorded.</p> : <ul className="workbench-list">{changes.map((change) => <li key={change.path}><details><summary><strong>{change.path}</strong> <span>{change.status} +{change.additions} −{change.deletions}</span></summary>{change.diff ? <pre tabIndex={0}>{change.diff}</pre> : <p>Diff receipt is not available.</p>}</details></li>)}</ul>}</div>
        <div><h4>Tests</h4>{tests.length === 0 ? <p>No tests have run.</p> : <ul className="workbench-list">{tests.map((test) => <li key={test.name}><strong>{test.name}</strong><span>{test.status}{test.durationMs == null ? '' : ` / ${test.durationMs} ms`}</span>{test.details ? <small>{test.details}</small> : null}</li>)}</ul>}</div>
      </div>
    </section>
  )
}
