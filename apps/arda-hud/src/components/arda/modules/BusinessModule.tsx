// sigil: REPAIR
import { BriefcaseBusiness, Building2, FileJson, Landmark } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import LineList from '../primitives/LineList'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { primarySigilForSource } from '../../../lib/soterionRender'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'
import SourceFreshnessStrip from './SourceFreshnessStrip'
import OpportunityBoard from '../../business/OpportunityBoard'
import CommitmentLedger from '../../business/CommitmentLedger'
import ExperimentPanel from '../../business/ExperimentPanel'
import ValueEvidencePanel from '../../business/ValueEvidencePanel'
import {
  emptyCompanyOpsSnapshot,
  highestValueAction,
  type CompanyOpsSnapshot,
} from '../../../lib/companyOps'

interface BusinessModuleProps {
  mode: string
  clientCount: number
  stateKeyCount: number
  companyViewTitle: string
  companyViewPreview: string
  clientPaths: string[]
  stateKeys: string[]
  sourceCoverage?: SourceCoverageBadgeState
  sourceProvenance?: ArdaSourceProvenance[]
  companyOps?: CompanyOpsSnapshot
  tag?: string
}

export default function BusinessModule({
  mode,
  clientCount,
  stateKeyCount,
  companyViewTitle,
  companyViewPreview,
  clientPaths,
  stateKeys,
  sourceCoverage,
  sourceProvenance,
  companyOps = emptyCompanyOpsSnapshot,
  tag,
}: BusinessModuleProps) {
  const nextAction = highestValueAction(companyOps)
  const activeEngagements = companyOps.engagements.filter((engagement) =>
    ['won', 'delivered', 'invoiced', 'paid'].includes(engagement.state),
  )
  return (
    <ModuleCard
      title="Business"
      eyebrow="Operations and product context"
      marker={primarySigilForSource('prometheus')}
      accent="gold"
      tag={tag}
      actions={<SourceCoverageBadge coverage={sourceCoverage} />}
    >
      <div className="split-stack">
        <section className="company-ops__next" aria-labelledby="company-next-action">
          <h3 id="company-next-action">Highest-value next operator action</h3>
          <strong>{nextAction?.next_action ?? 'Review evidence before creating commercial work.'}</strong>
          <p>{nextAction
            ? `${nextAction.title} · score ${nextAction.score.toFixed(2)} · ${Object.entries(nextAction.score_components).map(([key, value]) => `${key.replace(/_/g, ' ')} ${value.toFixed(2)}`).join(', ')}`
            : 'No scored opportunity is currently available.'}</p>
        </section>

        <div className="company-ops__grid">
          <section aria-labelledby="company-active-work">
            <h3 id="company-active-work">Active paid and client work</h3>
            {activeEngagements.length === 0 ? <p>No active client engagements.</p> : (
              <ul className="company-ops__list">{activeEngagements.map((engagement) => (
                <li key={engagement.engagement_id}>
                  <strong>{engagement.title}</strong>
                  <span className={`company-stage company-stage--${engagement.state}`}>{engagement.state}</span>
                  <small>{engagement.realized_value
                    ? `Realized value backed by receipt ${engagement.realized_value.outcome_receipt_id}`
                    : 'Forecast only; no realized-value receipt.'}</small>
                </li>
              ))}</ul>
            )}
          </section>
          <CommitmentLedger commitments={companyOps.commitments} />
          <OpportunityBoard opportunities={companyOps.opportunities} />
          <ExperimentPanel experiments={companyOps.experiments} />
          <section aria-labelledby="company-drafts">
            <h3 id="company-drafts">Drafts awaiting approval</h3>
            {companyOps.drafts.length === 0 ? <p>No external drafts awaiting approval.</p> : (
              <ul className="company-ops__list">{companyOps.drafts.map((draft) => (
                <li key={draft.proposal_id}><strong>{draft.title}</strong><span>{draft.audience}</span><small>{draft.approval_required ? 'Explicit operator approval required' : 'Review required'}</small></li>
              ))}</ul>
            )}
          </section>
        </div>
        <ValueEvidencePanel snapshot={companyOps} />

        <div className="overview-grid">
          <div className="overview-callout">
            <BriefcaseBusiness size={18} />
            <div>
              <div className="overview-callout__label">Mode</div>
              <strong>{mode}</strong>
            </div>
          </div>
          <div className="overview-callout">
            <Building2 size={18} />
            <div>
              <div className="overview-callout__label">Client Records</div>
              <strong>{clientCount}</strong>
            </div>
          </div>
          <div className="overview-callout">
            <FileJson size={18} />
            <div>
              <div className="overview-callout__label">State Keys</div>
              <strong>{stateKeyCount}</strong>
            </div>
          </div>
        </div>

        <div className="document-list compact">
          <article className="document-list__item">
            <strong>{companyViewTitle}</strong>
            <p>{companyViewPreview}</p>
          </article>
        </div>

        <SourceFreshnessStrip
          title="Business Source Freshness"
          records={sourceProvenance}
          terms={['business', 'client']}
        />

        <div className="split-stack">
          <div>
            <div className="module-subtitle">
              <Landmark size={14} /> Client Paths
            </div>
            <LineList
              items={clientPaths.map((path) => ({
                label: path.split('/').slice(-2).join('/'),
                value: path,
              }))}
            />
          </div>
          <div>
            <div className="module-subtitle">
              <FileJson size={14} /> State Keys
            </div>
            <LineList
              items={stateKeys.map((key) => ({
                label: key,
                value: 'available',
              }))}
            />
          </div>
        </div>
      </div>
    </ModuleCard>
  )
}
