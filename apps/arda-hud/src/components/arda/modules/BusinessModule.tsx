// sigil: REPAIR
import { BriefcaseBusiness, Building2, FileJson, Landmark } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import LineList from '../primitives/LineList'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { primarySigilForSource } from '../../../lib/soterionRender'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'
import SourceFreshnessStrip from './SourceFreshnessStrip'

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
  tag,
}: BusinessModuleProps) {
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
