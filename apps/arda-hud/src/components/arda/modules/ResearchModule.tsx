import { FormEvent, useEffect, useMemo, useState } from 'react'
import ModuleCard from '../ModuleCard'
import ResearchCitationDrawer from './ResearchCitationDrawer'
import {
  changeResearchWatchlistState,
  createMutationEnvelope,
  createResearchQuestion,
  createResearchWatchlist,
  formatCadence,
  loadResearchOperatorId,
  listResearchBriefs,
  listResearchQuestions,
  listResearchWatchlists,
  newQuestionDraft,
  newWatchlistDraft,
  projectResearchState,
  type ResearchBrief,
  type ResearchQuestion,
  type ResearchWatchlist,
} from '../../../lib/research'

function errorMessage(value: unknown): string { return value instanceof Error ? value.message : String(value) }

export default function ResearchModule() {
  const [question, setQuestion] = useState<ResearchQuestion>(() => newQuestionDraft())
  const [watchlist, setWatchlist] = useState<ResearchWatchlist>(() => newWatchlistDraft())
  const [questions, setQuestions] = useState<ResearchQuestion[]>([])
  const [watchlists, setWatchlists] = useState<ResearchWatchlist[]>([])
  const [briefs, setBriefs] = useState<ResearchBrief[]>([])
  const [selectedBriefId, setSelectedBriefId] = useState<string | null>(null)
  const [proposalId, setProposalId] = useState('hud-research')
  const [approvalId, setApprovalId] = useState('hud-research-operator')
  const [busy, setBusy] = useState(false)
  const [operatorId, setOperatorId] = useState<string | null>(null)
  const [message, setMessage] = useState('Research remains advisory; approved knowledge and proposals are separate states.')
  const [error, setError] = useState<string | null>(null)

  const selectedBrief = briefs.find((brief) => brief.brief_id === selectedBriefId) ?? briefs[0] ?? null
  const selectedWatchlist = watchlists[0] ?? null
  const questionIds = useMemo(() => new Set(questions.map((item) => item.question_id)), [questions])
  const run = async (operation: () => Promise<void>) => {
    setBusy(true); setError(null)
    try { await operation() } catch (caught) { setError(errorMessage(caught)) } finally { setBusy(false) }
  }
  const refresh = () => void run(async () => {
    const configuredOperator = operatorId ?? await loadResearchOperatorId()
    setOperatorId(configuredOperator)
    const [questionResponse, watchlistResponse, briefResponse] = await Promise.all([listResearchQuestions(configuredOperator), listResearchWatchlists(configuredOperator), listResearchBriefs(configuredOperator)])
    setQuestions(questionResponse.questions); setWatchlists(watchlistResponse.watchlists); setBriefs(briefResponse.briefs)
    setMessage('Research workspace refreshed from the typed harness projection.')
  })
  useEffect(() => { refresh() }, [])

  const submitQuestion = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      if (!operatorId) throw new Error('Research operator identity is not loaded')
      const response = await createResearchQuestion(operatorId, { ...question, owner: operatorId }, createMutationEnvelope(proposalId, approvalId, 'question'))
      setQuestions((current) => [...current.filter((item) => item.question_id !== response.question.question_id), response.question])
      setWatchlist((current) => ({ ...current, question_ids: current.question_ids.includes(response.question.question_id) ? current.question_ids : [...current.question_ids, response.question.question_id] }))
      setQuestion(newQuestionDraft())
      setMessage(`Question composed. Backend status: ${response.backend_status ?? 'advisory receipt recorded'}.`)
    })
  }
  const submitWatchlist = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      if (!watchlist.question_ids.length) throw new Error('Select at least one composed question before creating a watchlist')
      if (!operatorId) throw new Error('Research operator identity is not loaded')
      const created = await createResearchWatchlist(operatorId, watchlist, createMutationEnvelope(proposalId, approvalId, 'watchlist'))
      setWatchlists((current) => [...current.filter((item) => item.watchlist_id !== created.watchlist_id), created])
      setMessage(`Watchlist ${created.name || created.watchlist_id} created with ${created.question_ids.length} bounded question(s).`)
    })
  }
  const changeState = (action: 'pause' | 'resume' | 'retire') => void run(async () => {
    if (!selectedWatchlist) throw new Error('Create or refresh a watchlist before changing its state')
    if (!operatorId) throw new Error('Research operator identity is not loaded')
    const changed = await changeResearchWatchlistState(operatorId, selectedWatchlist.watchlist_id, action, createMutationEnvelope(proposalId, approvalId, action))
    setWatchlists((current) => current.map((item) => item.watchlist_id === changed.watchlist_id ? changed : item))
    setMessage(`Watchlist ${action} receipt recorded. Pause is immediately available.`)
  })

  return <ModuleCard title="Research" eyebrow="Bounded evidence workspace" accent="gold">
    <div className="research-workspace" aria-label="ARDA research workspace">
      <div className="research-toolbar">
        <div><span className="research-eyebrow">Warden → Varda → advisory brief</span><p>{message}</p></div>
        <button type="button" onClick={refresh} disabled={busy}>Refresh workspace</button>
      </div>
      {error ? <p className="research-error" role="alert">{error}</p> : null}
      <section className="research-panel" aria-labelledby="research-question-title">
        <header className="research-section-heading"><h3 id="research-question-title">Compose explicit question</h3><span>bounded request</span></header>
        <form className="research-form" onSubmit={submitQuestion}>
          <label>Question<textarea required value={question.question} onChange={(event) => setQuestion({ ...question, question: event.target.value })} rows={3} placeholder="What should Warden investigate?" /></label>
          <label>Rationale<textarea required value={question.rationale} onChange={(event) => setQuestion({ ...question, rationale: event.target.value })} rows={2} placeholder="Why is this bounded research useful?" /></label>
          <div className="research-form-grid"><label>Owner<input required value={question.owner} onChange={(event) => setQuestion({ ...question, owner: event.target.value })} /></label><label>Max sources<input type="number" min="1" max="50" value={question.source_policy.max_sources_per_run} onChange={(event) => setQuestion({ ...question, source_policy: { ...question.source_policy, max_sources_per_run: Number(event.target.value) } })} /></label><label>Max results<input type="number" min="1" max="100" value={question.budgets.max_results} onChange={(event) => setQuestion({ ...question, budgets: { ...question.budgets, max_results: Number(event.target.value) } })} /></label></div>
          <button type="submit" disabled={busy}>Create bounded question</button>
        </form>
      </section>
      <section className="research-panel" aria-labelledby="research-watchlist-title">
        <header className="research-section-heading"><h3 id="research-watchlist-title">Compose watchlist</h3><span>{selectedWatchlist ? selectedWatchlist.state : 'not created'}</span></header>
        <form className="research-form" onSubmit={submitWatchlist}>
          <label>Watchlist name<input required value={watchlist.name} onChange={(event) => setWatchlist({ ...watchlist, name: event.target.value })} placeholder="Security notices" /></label>
          <fieldset><legend>Questions in watchlist</legend>{questions.length === 0 ? <p className="research-muted">Create a question first.</p> : questions.map((item) => <label className="research-check" key={item.question_id}><input type="checkbox" checked={watchlist.question_ids.includes(item.question_id)} onChange={(event) => setWatchlist({ ...watchlist, question_ids: event.target.checked ? [...watchlist.question_ids, item.question_id] : watchlist.question_ids.filter((id) => id !== item.question_id) })} />{item.question}</label>)}</fieldset>
          <button type="submit" disabled={busy || questions.length === 0}>Create bounded watchlist</button>
        </form>
        {selectedWatchlist ? <div className="research-watchlist-controls"><span>Cadence is controlled by the backend; current questions: {selectedWatchlist.question_ids.filter((id) => questionIds.has(id)).length}</span><button type="button" disabled={busy || selectedWatchlist.state === 'retired'} onClick={() => changeState('pause')}>Pause now</button><button type="button" disabled={busy || selectedWatchlist.state !== 'paused'} onClick={() => changeState('resume')}>Resume</button></div> : null}
      </section>
      <section className="research-panel" aria-labelledby="research-brief-title">
        <header className="research-section-heading"><h3 id="research-brief-title">Brief states and evidence</h3><span>{briefs.length} brief(s)</span></header>
        <div className="research-brief-picker" role="listbox" aria-label="Research briefs">{briefs.length === 0 ? <p className="research-muted">No durable briefs are available yet.</p> : briefs.map((brief) => <button type="button" role="option" aria-selected={selectedBrief?.brief_id === brief.brief_id} key={brief.brief_id} onClick={() => setSelectedBriefId(brief.brief_id)}>{brief.question || brief.brief_id}<small>{brief.stale ? 'stale' : 'current'} · {brief.citations?.length ?? 0} citations</small></button>)}</div>
        {selectedBrief ? <div className="research-brief">
          <p className="research-summary">{selectedBrief.executive_summary || 'No executive summary was recorded.'}</p>
          <div className="research-state-legend">{['preview', 'fetched', 'evaluated', 'approved', 'proposal'].map((state) => { const projection = projectResearchState(state); return <span className={`research-state research-state--${projection.tone}`} key={state}><strong>{projection.label}</strong><small>{projection.description}</small></span> })}</div>
          <dl className="research-metrics"><div><dt>Next cadence</dt><dd>{selectedWatchlist ? formatCadence(questions.find((item) => selectedWatchlist.question_ids.includes(item.question_id))?.cadence ?? { kind: 'manual' }) : 'Manual'}</dd></div><div><dt>Budget</dt><dd>{questions[0]?.budgets.max_fetch_bytes.toLocaleString() ?? 'Not declared'} bytes</dd></div><div><dt>Freshness</dt><dd>{selectedBrief.stale ? 'Stale — refresh required' : 'Within declared expiry'}</dd></div><div><dt>Backend hold</dt><dd>{selectedBrief.next_proposal?.length ? 'Proposal-only next step' : 'No hold reported'}</dd></div></dl>
          {selectedBrief.rumil_evidence ? <section className="research-citations" aria-label="Rúmil audit evidence">
            <header className="research-section-heading"><h3>Rúmil audit evidence</h3><span>{selectedBrief.rumil_evidence.completeness}</span></header>
            <dl className="research-metrics">
              <div><dt>Evaluation</dt><dd>{selectedBrief.rumil_evidence.evaluation_status.replace(/_/g, ' ')}</dd></div>
              <div><dt>Baseline</dt><dd>{selectedBrief.rumil_evidence.stale_baseline ? 'Stale baseline' : 'Current baseline'}</dd></div>
              <div><dt>Authority</dt><dd>Advisory only — execution disabled</dd></div>
              <div><dt>Packet</dt><dd>{selectedBrief.rumil_evidence.packet_reference}</dd></div>
            </dl>
            <p><strong>Evidence classes:</strong> {selectedBrief.rumil_evidence.evidence_classes.join(', ')}</p>
            {selectedBrief.rumil_evidence.rejected_providers.length ? <p><strong>Rejected providers:</strong> {selectedBrief.rumil_evidence.rejected_providers.join(', ')}</p> : null}
            {selectedBrief.rumil_evidence.missing_evidence.length ? <p><strong>Missing evidence:</strong> {selectedBrief.rumil_evidence.missing_evidence.join(', ')}</p> : null}
            {selectedBrief.rumil_evidence.degraded_reasons.length ? <p><strong>Coverage notes:</strong> {selectedBrief.rumil_evidence.degraded_reasons.join('; ')}</p> : null}
          </section> : null}
          <ResearchCitationDrawer citations={selectedBrief.citations ?? []} />
        </div> : null}
      </section>
      <div className="research-approval-fields"><label>Proposal ID<input value={proposalId} onChange={(event) => setProposalId(event.target.value)} /></label><label>Approval ID<input value={approvalId} onChange={(event) => setApprovalId(event.target.value)} /></label></div>
    </div>
  </ModuleCard>
}
