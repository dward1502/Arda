import { useEffect, useMemo, useState } from 'react'
import type { ContinuityHorizonId, ContinuityViewModel } from './viewModels'
import { createContinuityClient, type ContinuityProjection } from '../../lib/continuity'
import { loadConfiguredOperatorId } from '../../lib/personalOps'

interface ContinuityFocusedWorkstationViewProps {
  model: ContinuityViewModel | null
  session?: ContinuityProjection | null
}

function formatMinor(value: number, currency: string): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency }).format(value / 100)
}

export function ContinuityFocusedWorkstationView({ model, session = null }: ContinuityFocusedWorkstationViewProps) {
  const [horizon, setHorizon] = useState<ContinuityHorizonId>('human')
  const visibleItems = useMemo(() => model?.items.filter((item) => item.horizon === horizon) ?? [], [horizon, model])
  const [selectedId, setSelectedId] = useState<string | null>(visibleItems[0]?.id ?? null)
  const [handoffMessage, setHandoffMessage] = useState<string | null>(null)

  useEffect(() => {
    if (!visibleItems.some((item) => item.id === selectedId)) setSelectedId(visibleItems[0]?.id ?? null)
  }, [selectedId, visibleItems])

  if (!model) return <div className="continuity-focused-view continuity-focused-view--empty"><h3>Continuity projections unavailable</h3></div>

  const continueHere = async () => {
    if (!session?.handoff_id) return
    setHandoffMessage('Preparing continuity…')
    try {
      const operatorId = await loadConfiguredOperatorId()
      const suffix = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`
      const bytes = new TextEncoder().encode(`continue-here:${session.handoff_id}:${suffix}`)
      const digest = await globalThis.crypto.subtle.digest('SHA-256', bytes)
      const key = `sha256:${[...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('')}`
      await createContinuityClient(operatorId).continueHere(session.handoff_id, key)
      setHandoffMessage('Continuity accepted. Refreshing source truth…')
    } catch (error) {
      setHandoffMessage(error instanceof Error ? error.message : 'Continuity request failed')
    }
  }

  const selected = visibleItems.find((item) => item.id === selectedId) ?? null
  return (
    <div className={`continuity-focused-view continuity-focused-view--${model.status}`}>
      <header className="continuity-focused-view__hero">
        <div>
          <span>PRIVATE CONTINUITY SURFACE</span>
          <h3>Human + Business + Personal</h3>
          <p>{model.summary.join(' · ')}</p>
          <p data-testid="hermes-continuity-status">
            {session?.active
              ? `${session.surface_id ?? 'Hermes'} · ${session.freshness}`
              : 'No active Hermes session projection'}
          </p>
          {session?.private_refs_withheld ? <small>Private context withheld on this shared surface.</small> : null}
          {session?.action_ids.includes('continue_here') && session.handoff_id ? (
            <button onClick={() => void continueHere()} type="button">Continue here</button>
          ) : null}
          {handoffMessage ? <small role="status">{handoffMessage}</small> : null}
        </div>
        <div className="continuity-focused-view__value" aria-label="Business value truth">
          <span>Planned value</span>
          <b>{formatMinor(model.valueTruth.plannedMinor, model.valueTruth.currency)}</b>
          <span>Realized value</span>
          <b>{model.valueTruth.realizedReceiptCount > 0
            ? `${formatMinor(model.valueTruth.realizedMinor, model.valueTruth.currency)} · ${model.valueTruth.realizedReceiptCount} receipt${model.valueTruth.realizedReceiptCount === 1 ? '' : 's'}`
            : 'No receipt-backed realized value'}</b>
        </div>
      </header>

      <nav className="continuity-focused-view__horizons" aria-label="Continuity horizons">
        {model.horizons.map((entry) => (
          <button aria-pressed={horizon === entry.id} key={entry.id} onClick={() => setHorizon(entry.id)} type="button">
            <span>{entry.label}</span> <b>{entry.count}</b>{entry.attention > 0 ? <i>{entry.attention} missing</i> : null}
          </button>
        ))}
      </nav>

      <div className="continuity-focused-view__body">
        <section className="continuity-focused-view__rail" aria-label={`${horizon} continuity items`}>
          {visibleItems.map((item) => (
            <button aria-pressed={item.id === selectedId} key={item.id} onClick={() => setSelectedId(item.id)} type="button">
              <span>{item.kind}</span><b>{item.title}</b><small>{item.state}</small>
            </button>
          ))}
          {visibleItems.length === 0 ? <p>No current {horizon} items are projected.</p> : null}
        </section>

        <section className="continuity-focused-view__detail" aria-label="Selected continuity detail">
          {selected ? <>
            <span>{selected.kind} · {selected.state}</span>
            <h3>{selected.title}</h3>
            {selected.privateDetail ? <strong>Private focused detail</strong> : null}
            <p>{selected.summary}</p>
            {selected.path ? <code>{selected.path}</code> : null}
          </> : <p>Select an item to inspect its source-backed detail.</p>}
        </section>
      </div>

      <footer className="continuity-focused-view__footer arda-source-corner" aria-label="Continuity source truth">
        {model.sources.map((source) => <span data-truth-state={source.freshness.status} key={source.id}>{source.label}: {source.freshness.status}</span>)}
        {session ? <span data-truth-state={session.freshness}>Hermes lineage: {session.freshness}</span> : null}
      </footer>
    </div>
  )
}
