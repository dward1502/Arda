export type ResearchQuestionState = 'enabled' | 'paused' | 'retired'
export type ResearchWatchlistState = ResearchQuestionState
export type WatchlistCadence = { kind: 'manual' } | { kind: 'interval'; every_seconds: number }
export type ContradictionPolicy = 'require_disclosure' | 'block_approval'

export interface MutationEnvelope {
  approval: {
    schema_version: 'arda.orome.task_approval.v1'
    proposal_id: string
    approval_id: string
    ledger_writes: string[]
    decision: 'policy_safe'
    created_at_utc: string
  }
  idempotency_key: string
}

export interface ResearchQuestion {
  schema_version: string
  question_id: string
  owner: string
  question: string
  rationale: string
  tags: string[]
  cadence: WatchlistCadence
  expires_at_utc: string
  source_policy: {
    policy_id: string
    allowed_sources: string[]
    max_sources_per_run: number
    allow_private_targets: boolean
  }
  evidence_requirements: {
    minimum_canonical_sources: number
    require_canonical_fetch: boolean
    max_source_age_seconds: number
  }
  contradiction_policy: ContradictionPolicy
  budgets: {
    max_results: number
    max_fetch_bytes: number
    max_tokens: number
    max_attempts: number
  }
  notification_policy: { enabled: boolean; destination: string | null }
  state: ResearchQuestionState
  backend_suggestion_ids: string[]
}

export interface ResearchWatchlist {
  schema_version: string
  watchlist_id: string
  name: string
  question_ids: string[]
  state: ResearchWatchlistState
}

export interface ResearchBriefCitation {
  citation_id: string
  url: string
  title?: string
  source_identity?: string
  excerpt?: string
  fetch_status?: string
  rejection_reason?: string | null
  failure_reason?: string | null
  quality?: string
  freshness?: string
}

export interface ResearchBriefClaim {
  claim_id: string
  claim: string
  citation_ids: string[]
  confidence?: string
  support?: 'supporting' | 'opposing' | 'mixed' | 'unknown'
}

export interface ResearchBrief {
  schema_version: string
  brief_id: string
  question_id?: string
  question?: string
  scope?: string
  executive_summary?: string
  claims?: ResearchBriefClaim[]
  citations?: ResearchBriefCitation[]
  supporting_citation_ids?: string[]
  opposing_citation_ids?: string[]
  contradictions?: string[]
  uncertainty?: string[]
  missing_evidence?: string[]
  next_research?: string[]
  next_proposal?: string[]
  receipt_references?: string[]
  stale?: boolean
  expires_at_utc?: string
  material_fingerprint?: string
  no_change_receipt?: string
}

export interface BriefListResponse { briefs: ResearchBrief[] }
export interface QuestionListResponse { questions: ResearchQuestion[] }
export interface WatchlistListResponse { watchlists: ResearchWatchlist[] }
export interface QuestionCreateResponse {
  question: ResearchQuestion
  backend_suggestion?: { suggestion_id: string; authority: string; query: string }
  backend_status?: string
}

export interface ResearchSurfaceModel {
  label: string
  tone: 'neutral' | 'positive' | 'warning' | 'danger'
  description: string
}

export function createMutationEnvelope(proposalId: string, approvalId: string, action: string): MutationEnvelope {
  const stamp = Date.now()
  return {
    approval: {
      schema_version: 'arda.orome.task_approval.v1',
      proposal_id: proposalId.trim(),
      approval_id: approvalId.trim(),
      ledger_writes: [],
      decision: 'policy_safe',
      created_at_utc: new Date(stamp).toISOString(),
    },
    idempotency_key: `${action}-${stamp}`,
  }
}

export function newQuestionDraft(overrides: Partial<ResearchQuestion> = {}): ResearchQuestion {
  const id = globalThis.crypto?.randomUUID?.() ?? `question-${Date.now()}`
  return {
    schema_version: 'arda.warden.watchlist.v1',
    question_id: id,
    owner: 'operator',
    question: '',
    rationale: '',
    tags: [],
    cadence: { kind: 'manual' },
    expires_at_utc: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(),
    source_policy: { policy_id: 'public-web', allowed_sources: ['https://'], max_sources_per_run: 5, allow_private_targets: false },
    evidence_requirements: { minimum_canonical_sources: 1, require_canonical_fetch: true, max_source_age_seconds: 604800 },
    contradiction_policy: 'require_disclosure',
    budgets: { max_results: 10, max_fetch_bytes: 2_000_000, max_tokens: 4_000, max_attempts: 2 },
    notification_policy: { enabled: false, destination: null },
    state: 'enabled',
    backend_suggestion_ids: [],
    ...overrides,
  }
}

export function newWatchlistDraft(questionIds: string[] = []): ResearchWatchlist {
  const id = globalThis.crypto?.randomUUID?.() ?? `watchlist-${Date.now()}`
  return { schema_version: 'arda.warden.watchlist.v1', watchlist_id: id, name: '', question_ids: questionIds, state: 'enabled' }
}

export function projectResearchState(value: unknown): ResearchSurfaceModel {
  const state = typeof value === 'string' ? value : 'unknown'
  if (state === 'preview') return { label: 'Preview', tone: 'warning', description: 'Untrusted search preview; not evidence.' }
  if (state === 'fetched') return { label: 'Fetched source', tone: 'positive', description: 'Canonical source fetched and available for evaluation.' }
  if (state === 'evaluated') return { label: 'Evaluation', tone: 'positive', description: 'Varda evaluation is recorded; approval is separate.' }
  if (state === 'approved') return { label: 'Approved knowledge', tone: 'positive', description: 'Approved continuity may be used by governed consumers.' }
  if (state === 'proposal') return { label: 'Proposal', tone: 'warning', description: 'A governed next action is proposed, not executable.' }
  if (state === 'failed' || state === 'rejected') return { label: state === 'failed' ? 'Failed' : 'Rejected', tone: 'danger', description: 'This source or transition is not usable.' }
  return { label: state.replace(/_/g, ' '), tone: 'neutral', description: 'Research lifecycle state.' }
}

export function formatCadence(cadence: WatchlistCadence): string {
  return cadence.kind === 'manual' ? 'Manual' : `Every ${Math.max(1, Math.round(cadence.every_seconds / 3600))}h`
}

export function buildResearchUrl(path: string, base = import.meta.env.VITE_ARDA_HARNESS_URL ?? 'http://127.0.0.1:7878'): string {
  return `${base.replace(/\/$/, '')}${path}`
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(buildResearchUrl(path), { ...init, headers: { Accept: 'application/json', 'Content-Type': 'application/json', ...init?.headers } })
  if (!response.ok) throw new Error(`Research request failed (${response.status}): ${await response.text()}`)
  return response.json() as Promise<T>
}

export const listResearchQuestions = () => requestJson<QuestionListResponse>('/v1/research/questions')
export const listResearchWatchlists = () => requestJson<WatchlistListResponse>('/v1/research/watchlists')
export const listResearchBriefs = () => requestJson<BriefListResponse>('/v1/research/briefs')
export const createResearchQuestion = (question: ResearchQuestion, envelope: MutationEnvelope, readOnly = false) => requestJson<QuestionCreateResponse>('/v1/research/questions', { method: 'POST', body: JSON.stringify({ question, read_only: readOnly, envelope }) })
export const createResearchWatchlist = (watchlist: ResearchWatchlist, envelope: MutationEnvelope) => requestJson<ResearchWatchlist>('/v1/research/watchlists', { method: 'POST', body: JSON.stringify({ watchlist, envelope }) })
export const changeResearchWatchlistState = (id: string, action: 'pause' | 'resume' | 'retire', envelope: MutationEnvelope) => requestJson<ResearchWatchlist>(`/v1/research/watchlists/${encodeURIComponent(id)}/${action}`, { method: 'POST', body: JSON.stringify({ envelope }) })
