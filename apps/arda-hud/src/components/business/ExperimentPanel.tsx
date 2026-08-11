import type { CompanyExperiment } from '../../lib/companyOps'

export default function ExperimentPanel({ experiments }: { experiments: CompanyExperiment[] }) {
  return <section aria-labelledby="company-experiments"><h3 id="company-experiments">Revenue experiments</h3>
    {experiments.length === 0 ? <p>No bounded paid experiments.</p> : <ul className="company-ops__list">{experiments.map((item) => <li key={item.experiment_id}><strong>{item.title}</strong><span className="company-stage">{item.status}</span><span>{item.operator_hours_used}/{item.operator_hours_max} operator hours</span><small>Success: {item.success_threshold} · Stop: {item.stop_condition}</small></li>)}</ul>}
  </section>
}
