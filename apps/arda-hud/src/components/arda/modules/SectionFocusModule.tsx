// sigil: REPAIR
import ModuleCard from '../ModuleCard'
import LineList from '../primitives/LineList'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { primarySigilForSource } from '../../../lib/soterionRender'
import { DataFreshnessBadge } from './DataFreshnessBadge'
import { SourceRefreshAffordance } from './SourceRefreshAffordance'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'

interface SectionFocusModuleProps {
  title: string
  eyebrow: string
  accent: 'gold' | 'cyan' | 'ember' | 'mint' | 'violet'
  status: string
  owner: string
  panelCount: number
  sourceCount: number
  panels: string[]
  primarySources: string[]
  sourceCoverage?: SourceCoverageBadgeState
  sourceProvenance?: ArdaSourceProvenance[]
  tag?: string
}

function provenanceForPrimarySources(
  records: ArdaSourceProvenance[] | undefined,
  primarySources: string[],
): ArdaSourceProvenance[] {
  if (!records?.length || primarySources.length === 0) return []
  return records.filter((record) => record.sourcePaths.some((sourcePath) => (
    primarySources.some((primarySource) => sourcePath.includes(primarySource) || primarySource.includes(sourcePath))
  )))
}

export default function SectionFocusModule({
  title,
  eyebrow,
  accent,
  status,
  owner,
  panelCount,
  sourceCount,
  panels,
  primarySources,
  sourceCoverage,
  sourceProvenance,
  tag,
}: SectionFocusModuleProps) {
  const marker = primarySigilForSource(owner)
  const provenanceRecords = provenanceForPrimarySources(sourceProvenance, primarySources)
  return (
    <ModuleCard title={title} eyebrow={eyebrow} marker={marker} accent={accent} tag={tag} actions={<SourceCoverageBadge coverage={sourceCoverage} />}>
      <div className="section-focus">
        <LineList
          items={[
            { label: 'Status', value: status },
            { label: 'Owner', value: owner },
            { label: 'Panels', value: `${panelCount}` },
            { label: 'Primary Sources', value: `${sourceCount}` },
          ]}
        />
        <div className="token-cloud">
          {panels.map((panel) => (
            <span className="token-chip" key={panel}>{panel}</span>
          ))}
        </div>
        <div className="path-list">
          {primarySources.map((sourcePath) => (
            <div className="path-list__item" key={sourcePath}>{sourcePath}</div>
          ))}
        </div>
        {provenanceRecords.length > 0 ? (
          <div className="source-provenance-list">
            <div className="module-subtitle">Source Freshness</div>
            {provenanceRecords.slice(0, 4).map((record) => (
              <article className="source-provenance-list__item" key={record.domainId}>
                <div>
                  <strong>{record.label}</strong>
                  <span>{record.sourceKind} / {record.generatedAtUtc ?? record.observedAtUtc ?? 'timestamp unknown'}</span>
                </div>
                <span className="source-provenance-list__actions">
                  <SourceRefreshAffordance record={record} compact />
                  <DataFreshnessBadge record={record} compact />
                </span>
              </article>
            ))}
          </div>
        ) : null}
      </div>
    </ModuleCard>
  )
}
