import type { CompanyOpportunity } from '../../lib/companyOps'
import { formatMoney } from '../../lib/companyOps'

export default function OpportunityBoard({ opportunities }: { opportunities: CompanyOpportunity[] }) {
  return <section aria-labelledby="company-opportunities"><h3 id="company-opportunities">Opportunity board</h3>
    {opportunities.length === 0 ? <p>No qualified opportunities.</p> : <ol className="company-ops__list">{opportunities.map((item) => <li key={item.opportunity_id}>
      <strong>{item.title}</strong><span className={`company-stage company-stage--${item.stage}`}>{item.stage}</span>
      <span>{formatMoney(item.expected_value.currency, item.expected_value.low)}–{formatMoney(item.expected_value.currency, item.expected_value.high)} forecast · {Math.round(item.expected_value.confidence * 100)}% confidence</span>
      <small>Assumptions: {item.evidence.map((entry) => entry.assumption).join('; ') || 'not supplied'} · Evidence: {item.evidence.map((entry) => entry.citation).join('; ') || 'not supplied'}</small>
    </li>)}</ol>}
  </section>
}
