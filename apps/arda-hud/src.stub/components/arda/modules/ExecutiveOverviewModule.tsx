// sigil: REPAIR
import { Activity, HeartHandshake, Shield, TerminalSquare } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import { primarySigilForSource } from '../../../lib/soterionRender'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'

interface ExecutiveOverviewModuleProps {
  authority: string
  loveEquation: string
  activeTasks: string
  schemaVersion: string
  sourceCoverage?: SourceCoverageBadgeState
  tag?: string
}

export default function ExecutiveOverviewModule({
  authority,
  loveEquation,
  activeTasks,
  schemaVersion,
  sourceCoverage,
  tag,
}: ExecutiveOverviewModuleProps) {
  return (
    <ModuleCard
      title="Executive Overview"
      eyebrow="Sovereign world"
      marker={primarySigilForSource('prometheus')}
      accent="gold"
      tag={tag}
      actions={<SourceCoverageBadge coverage={sourceCoverage} />}
    >
      <div className="overview-grid">
        <div className="overview-callout">
          <Shield size={18} />
          <div>
            <div className="overview-callout__label">Authority</div>
            <strong>{authority}</strong>
          </div>
        </div>
        <div className="overview-callout">
          <HeartHandshake size={18} />
          <div>
            <div className="overview-callout__label">Love Equation</div>
            <strong>{loveEquation}</strong>
          </div>
        </div>
        <div className="overview-callout">
          <Activity size={18} />
          <div>
            <div className="overview-callout__label">Active Tasks</div>
            <strong>{activeTasks}</strong>
          </div>
        </div>
        <div className="overview-callout">
          <TerminalSquare size={18} />
          <div>
            <div className="overview-callout__label">Schema</div>
            <strong>{schemaVersion}</strong>
          </div>
        </div>
      </div>
    </ModuleCard>
  )
}
