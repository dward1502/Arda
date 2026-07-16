// sigil: REPAIR
export interface RoutableProviderModel {
  id: string
  contextWindow: number | null
  healthy: boolean
  isDefault: boolean
  capableTasks: string[]
}

export interface RoutableProviderEntry {
  providerId: string
  providerName: string
  accessTier: string
  qualityBand: string
  enabled: boolean
  healthy: boolean
  models: RoutableProviderModel[]
  avgLatencyMs: number | null
  activeConnections: number
}

interface RoutableProvidersPanelProps {
  providers: RoutableProviderEntry[]
}

function formatContextWindow(contextWindow: number | null): string {
  if (!contextWindow) return 'ctx unknown'
  if (contextWindow >= 1000) return `${Math.round(contextWindow / 1000)}k ctx`
  return `${contextWindow} ctx`
}

export default function RoutableProvidersPanel({ providers }: RoutableProvidersPanelProps) {
  return (
    <section className="systems-panel systems-panel--providers">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">Inference Mesh</div>
        <h3 className="systems-panel__title">Routable Providers</h3>
      </header>
      <div className="provider-grid">
        {providers.map((provider) => (
          <article className="provider-card" key={provider.providerId}>
            <div className="provider-card__header">
              <strong>{provider.providerName || provider.providerId}</strong>
              <span>{provider.activeConnections} inflight</span>
            </div>
            <div className="provider-card__latency">
              {provider.avgLatencyMs ? `${provider.avgLatencyMs} ms ewma` : 'latency learning'}
            </div>
            <p>
              {provider.providerId} / {provider.accessTier} / {provider.qualityBand} /{' '}
              {provider.enabled ? 'enabled' : 'disabled'} / {provider.healthy ? 'healthy' : 'degraded'}
            </p>
            <div className="systems-chip-cloud">
              {provider.models.map((model) => (
                <span className="systems-chip systems-chip--accent" key={`${provider.providerId}-${model.id}`}>
                  {model.id} / {formatContextWindow(model.contextWindow)} /{' '}
                  {model.healthy ? 'healthy' : 'degraded'}{model.isDefault ? ' / default' : ''}
                  {model.capableTasks.length > 0 ? ` / ${model.capableTasks.join(', ')}` : ''}
                </span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </section>
  )
}
