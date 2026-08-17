export type ContinuityFreshness = 'fresh' | 'stale' | 'unavailable'
export type ContinuityPrivacyClass = 'public_room' | 'shared_room' | 'private_room' | 'personal_device'
export type HandoffState = 'requested' | 'prepared' | 'accepted' | 'active' | 'declined' | 'expired' | 'failed'

export interface ContinuityProjection {
  schema_version: 'arda.continuity-projection.v1'
  generated_at: string
  active: boolean
  session_lineage_id: string | null
  current_session_id: string | null
  surface_id: string | null
  privacy_class: ContinuityPrivacyClass | null
  freshness: ContinuityFreshness
  handoff_id: string | null
  handoff_state: HandoffState | null
  action_ids: string[]
  private_refs_withheld: boolean
  topic_refs: string[]
  commitment_refs: string[]
  memory_scope_refs: string[]
}

const DEFAULT_HARNESS = 'http://127.0.0.1:7878'
const PRIVACY = new Set<ContinuityPrivacyClass>([
  'public_room', 'shared_room', 'private_room', 'personal_device',
])
const FRESHNESS = new Set<ContinuityFreshness>(['fresh', 'stale', 'unavailable'])
const HANDOFF_STATES = new Set<HandoffState>([
  'requested', 'prepared', 'accepted', 'active', 'declined', 'expired', 'failed',
])

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('invalid continuity projection')
  return value as Record<string, unknown>
}

function optionalString(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null
}

function strings(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : []
}

export function parseContinuityProjection(value: unknown): ContinuityProjection {
  const item = record(value)
  if (item.schema_version !== 'arda.continuity-projection.v1') throw new Error('unsupported continuity projection schema')
  const freshness = String(item.freshness ?? '') as ContinuityFreshness
  if (!FRESHNESS.has(freshness)) throw new Error('invalid continuity freshness')
  const privacy = optionalString(item.privacy_class) as ContinuityPrivacyClass | null
  if (privacy && !PRIVACY.has(privacy)) throw new Error('invalid continuity privacy class')
  const handoffState = optionalString(item.handoff_state) as HandoffState | null
  if (handoffState && !HANDOFF_STATES.has(handoffState)) throw new Error('invalid handoff state')
  const privateRefsWithheld = item.private_refs_withheld === true
  const topicRefs = strings(item.topic_refs)
  const commitmentRefs = strings(item.commitment_refs)
  const memoryScopeRefs = strings(item.memory_scope_refs)
  if (privateRefsWithheld && (topicRefs.length || commitmentRefs.length || memoryScopeRefs.length)) {
    throw new Error('private continuity references were not withheld')
  }
  return {
    schema_version: 'arda.continuity-projection.v1',
    generated_at: String(item.generated_at ?? ''),
    active: item.active === true,
    session_lineage_id: optionalString(item.session_lineage_id),
    current_session_id: optionalString(item.current_session_id),
    surface_id: optionalString(item.surface_id),
    privacy_class: privacy,
    freshness,
    handoff_id: optionalString(item.handoff_id),
    handoff_state: handoffState,
    action_ids: strings(item.action_ids),
    private_refs_withheld: privateRefsWithheld,
    topic_refs: topicRefs,
    commitment_refs: commitmentRefs,
    memory_scope_refs: memoryScopeRefs,
  }
}

function baseUrl(base: string): string {
  return base.replace(/\/$/, '')
}

export async function loadContinuityProjection(
  operatorId: string,
  base = import.meta.env.VITE_ARDA_HARNESS_URL ?? DEFAULT_HARNESS,
): Promise<ContinuityProjection> {
  const response = await fetch(`${baseUrl(base)}/v1/continuity/projection`, {
    headers: { 'x-arda-operator-id': operatorId.trim() },
  })
  if (!response.ok) throw new Error(`continuity projection failed: ${response.status}`)
  return parseContinuityProjection(await response.json())
}

export function createContinuityClient(
  operatorId: string,
  base = import.meta.env.VITE_ARDA_HARNESS_URL ?? DEFAULT_HARNESS,
) {
  const operator_ref = operatorId.trim()
  if (!operator_ref) throw new Error('configured operator identity is required')
  return {
    async continueHere(handoffId: string, idempotencyKey: string): Promise<unknown> {
      if (!handoffId.trim()) throw new Error('handoff identity is required')
      const response = await fetch(
        `${baseUrl(base)}/v1/handoffs/${encodeURIComponent(handoffId)}/accept`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ operator_ref, idempotency_key: idempotencyKey }),
        },
      )
      if (!response.ok) throw new Error(`continue here failed: ${response.status}`)
      return response.json()
    },
  }
}
