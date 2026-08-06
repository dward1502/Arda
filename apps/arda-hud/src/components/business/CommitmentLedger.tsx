import type { CompanyCommitment } from '../../lib/companyOps'

export default function CommitmentLedger({ commitments }: { commitments: CompanyCommitment[] }) {
  const ordered = [...commitments].sort((a, b) => a.due_at.localeCompare(b.due_at))
  return <section aria-labelledby="company-commitments"><h3 id="company-commitments">Commitments due soon</h3>
    {ordered.length === 0 ? <p>No approved commitments due.</p> : <ul className="company-ops__list">{ordered.map((item) => <li key={item.commitment_id}><strong>{item.title}</strong><span className={`company-stage company-stage--${item.status}`}>{item.status}</span><span>Due {new Date(item.due_at).toLocaleDateString()}</span><small>Approval receipt {item.approval_receipt_id}</small></li>)}</ul>}
  </section>
}
