import { formatMoney, type CompanyOpsSnapshot } from '../../lib/companyOps'

export default function ValueEvidencePanel({ snapshot }: { snapshot: CompanyOpsSnapshot }) {
  const expected = snapshot.expected_value
  return <section aria-labelledby="company-value"><h3 id="company-value">Expected versus realized value</h3>
    <dl className="company-ops__values">
      <div><dt>Forecast</dt><dd>{formatMoney(expected.currency, expected.low)}–{formatMoney(expected.currency, expected.high)} ({Math.round(expected.confidence * 100)}% confidence)</dd></div>
      <div><dt>Realized</dt><dd>{formatMoney(snapshot.realized_value.currency, snapshot.realized_value.amount)} · receipt {snapshot.realized_value.outcome_receipt_id}</dd></div>
      <div><dt>Consumption</dt><dd>{formatMoney(snapshot.cost.currency, snapshot.cost.amount)} · {snapshot.cost.operator_hours} operator hours</dd></div>
    </dl>
  </section>
}
