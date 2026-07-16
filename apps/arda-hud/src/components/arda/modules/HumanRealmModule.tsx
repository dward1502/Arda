// sigil: REPAIR
import { BookOpenText, Map, UserRound } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { fallbackProtocolMarker } from '../../../lib/soterionRender'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'
import SourceFreshnessStrip from './SourceFreshnessStrip'

interface HumanDocument {
  title: string
  path: string
  body_preview: string
}

interface HumanRealmModuleProps {
  docs: HumanDocument[]
  notes: HumanDocument[]
  planShelf: {
    humanPlanRoot: string
    plans: Array<{
      id: string
      title: string
      owner: string
      openTaskCount: number
      humanPlanPath: string
    }>
  }
  counts: {
    docs: number
    notes: number
    summaries: number
    arandur: number
  }
  sourceCoverage?: SourceCoverageBadgeState
  sourceProvenance?: ArdaSourceProvenance[]
  tag?: string
}

export default function HumanRealmModule({ docs, notes, planShelf, counts, sourceCoverage, sourceProvenance, tag }: HumanRealmModuleProps) {
  const reviewMarker = fallbackProtocolMarker('REVIEW')
  return (
    <ModuleCard
      title="Human Realm"
      eyebrow="Readable layer"
      marker={reviewMarker}
      accent="mint"
      tag={tag}
      actions={<SourceCoverageBadge coverage={sourceCoverage} />}
    >
      <div className="split-stack">
        <div>
          <div className="module-subtitle"><BookOpenText size={14} /> Docs</div>
          <div className="document-list">
            {docs.slice(0, 4).map((doc) => (
              <article className="document-list__item" key={doc.path}>
                <strong>{doc.title}</strong>
                <span>{doc.path}</span>
                <p>{doc.body_preview}</p>
              </article>
            ))}
          </div>
        </div>
        <div>
          <div className="module-subtitle"><UserRound size={14} /> Notes</div>
          <div className="token-cloud">
            <span className="token-chip">Docs {counts.docs}</span>
            <span className="token-chip">Notes {counts.notes}</span>
            <span className="token-chip">Summaries {counts.summaries}</span>
            <span className="token-chip">Arandur {counts.arandur}</span>
          </div>
          <div className="document-list">
            {notes.slice(0, 3).map((note) => (
              <article className="document-list__item" key={note.path}>
                <strong>{note.title}</strong>
                <span>{note.path}</span>
                <p>{note.body_preview}</p>
              </article>
            ))}
          </div>
        </div>
      </div>
      <div style={{ marginTop: 16 }}>
        <SourceFreshnessStrip
          title="Human Source Freshness"
          records={sourceProvenance}
          terms={['human', 'notes', 'docs', 'arandur']}
        />
      </div>
      <div style={{ marginTop: 16 }}>
        <div className="module-subtitle"><Map size={14} /> Plan Shelf</div>
        <div className="document-list">
          <article className="document-list__item">
            <strong>{planShelf.humanPlanRoot}</strong>
            <p>Central readable plan root for graph-linked operator and machine thought.</p>
          </article>
          {planShelf.plans.slice(0, 4).map((plan) => (
            <article className="document-list__item" key={plan.id}>
              <strong>{plan.title}</strong>
              <span>{plan.humanPlanPath}</span>
              <p>{plan.owner} / open tasks {plan.openTaskCount}</p>
            </article>
          ))}
        </div>
      </div>
    </ModuleCard>
  )
}
