// sigil: REPAIR
import ModuleCard from '../ModuleCard'
import MetricPill from '../primitives/MetricPill'
import { primarySigilForSource } from '../../../lib/soterionRender'

interface PresencePanelProps {
  tone: 'gold' | 'cyan' | 'ember' | 'mint' | 'violet'
  systemStatus: string
  sectionCount: number
  agentCount: number
}

export default function PresencePanel({
  tone,
  systemStatus,
  sectionCount,
  agentCount,
}: PresencePanelProps) {
  return (
    <ModuleCard
      title="Presence"
      eyebrow="Live status"
      marker={primarySigilForSource('prometheus')}
      accent={tone}
    >
      <div className="presence-block">
        <MetricPill label="System" value={systemStatus} />
        <MetricPill label="Sections" value={`${sectionCount}`} />
        <MetricPill label="Agents" value={`${agentCount}`} />
      </div>
    </ModuleCard>
  )
}
