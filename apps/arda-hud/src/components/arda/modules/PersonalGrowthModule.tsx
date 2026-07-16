// sigil: REPAIR
import { BookHeart, BrainCircuit, Heart, TimerReset } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import LineList from '../primitives/LineList'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { fallbackProtocolMarker } from '../../../lib/soterionRender'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'
import SourceFreshnessStrip from './SourceFreshnessStrip'

interface PersonalGrowthModuleProps {
  name: string
  role: string
  location: string
  priorities: string[]
  values: string[]
  researchDomains: string[]
  creativeDomains: string[]
  personalDocsTotal: number
  onboardPreview: string
  sourceCoverage?: SourceCoverageBadgeState
  sourceProvenance?: ArdaSourceProvenance[]
  tag?: string
}

export default function PersonalGrowthModule({
  name,
  role,
  location,
  priorities,
  values,
  researchDomains,
  creativeDomains,
  personalDocsTotal,
  onboardPreview,
  sourceCoverage,
  sourceProvenance,
  tag,
}: PersonalGrowthModuleProps) {
  return (
    <ModuleCard
      title="Personal Growth"
      eyebrow="Sovereign human context"
      marker={fallbackProtocolMarker('REVIEW')}
      accent="mint"
      tag={tag}
      actions={<SourceCoverageBadge coverage={sourceCoverage} />}
    >
      <div className="split-stack">
        <div className="overview-grid">
          <div className="overview-callout">
            <Heart size={18} />
            <div>
              <div className="overview-callout__label">Principal</div>
              <strong>{name}</strong>
            </div>
          </div>
          <div className="overview-callout">
            <TimerReset size={18} />
            <div>
              <div className="overview-callout__label">Personal Docs</div>
              <strong>{personalDocsTotal}</strong>
            </div>
          </div>
          <div className="overview-callout">
            <BookHeart size={18} />
            <div>
              <div className="overview-callout__label">Location</div>
              <strong>{location}</strong>
            </div>
          </div>
        </div>

        <div className="document-list compact">
          <article className="document-list__item">
            <strong>{role}</strong>
            <p>{onboardPreview}</p>
          </article>
        </div>

        <SourceFreshnessStrip
          title="Personal Source Freshness"
          records={sourceProvenance}
          terms={['personal', 'human_context']}
        />

        <div className="split-stack">
          <div>
            <div className="module-subtitle">
              <Heart size={14} /> Priorities
            </div>
            <LineList items={priorities.map((priority, index) => ({ label: `P${index + 1}`, value: priority }))} />
          </div>
          <div>
            <div className="module-subtitle">
              <BrainCircuit size={14} /> Values
            </div>
            <LineList items={values.map((value) => ({ label: value, value: 'active' }))} />
          </div>
          <div>
            <div className="module-subtitle">
              <BookHeart size={14} /> Research / Creative
            </div>
            <LineList
              items={[...researchDomains.slice(0, 3), ...creativeDomains.slice(0, 3)].map((entry) => ({
                label: entry,
                value: researchDomains.includes(entry) ? 'research' : 'creative',
              }))}
            />
          </div>
        </div>
      </div>
    </ModuleCard>
  )
}
