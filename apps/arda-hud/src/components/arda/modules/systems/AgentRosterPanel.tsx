// sigil: REPAIR
interface SystemAgent {
  name: string
  realm: string
  status: string
  trustScore: string
  sigil: string
}

interface AgentRosterPanelProps {
  agents: SystemAgent[]
}

export default function AgentRosterPanel({ agents }: AgentRosterPanelProps) {
  return (
    <section className="systems-panel systems-panel--agents">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">Agent Baseline</div>
        <h3 className="systems-panel__title">Agent Roster</h3>
      </header>
      <div className="agent-grid">
        {agents.map((agent) => (
          <article className="agent-card" key={agent.name}>
            <div className="agent-card__sigil">{agent.sigil}</div>
            <div>
              <div className="agent-card__name">{agent.name}</div>
              <div className="agent-card__realm">{agent.realm}</div>
            </div>
            <div className="agent-card__meta">
              <span>{agent.status}</span>
              <strong>{agent.trustScore}</strong>
            </div>
          </article>
        ))}
      </div>
    </section>
  )
}
