// sigil: REPAIR
interface LaneFitnessEntry {
  lane: string
  providerId: string
  avgLatencyMs: number | null
  successCount: number
  failureCount: number
}

interface LaneFitnessPanelProps {
  entries: LaneFitnessEntry[]
}

function reliability(successCount: number, failureCount: number): number {
  const total = successCount + failureCount
  if (total <= 0) return 0
  return successCount / total
}

export default function LaneFitnessPanel({ entries }: LaneFitnessPanelProps) {
  return (
    <section className="systems-panel systems-panel--fitness">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">Learned Preference</div>
        <h3 className="systems-panel__title">Lane Fitness</h3>
      </header>
      <div className="fitness-list">
        {entries.length > 0 ? (
          entries.map((entry) => (
            <article className="fitness-card" key={`${entry.lane}-${entry.providerId}`}>
              <div className="fitness-card__topline">
                <span className="fitness-card__lane">{entry.lane}</span>
                <span className="fitness-card__provider">{entry.providerId}</span>
              </div>
              <div className="fitness-card__metrics">
                <span>latency {entry.avgLatencyMs ? `${entry.avgLatencyMs} ms` : 'unknown'}</span>
                <span>success {entry.successCount}</span>
                <span>failure {entry.failureCount}</span>
                <strong>{Math.round(reliability(entry.successCount, entry.failureCount) * 100)}% reliable</strong>
              </div>
            </article>
          ))
        ) : (
          <div className="systems-empty">No lane memory yet.</div>
        )}
      </div>
    </section>
  )
}
