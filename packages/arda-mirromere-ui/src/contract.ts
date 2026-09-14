export const MIRROMERE_SURFACE_SCHEMA_VERSION = 'arda.mirromere.surface.v1' as const
export const MIRROMERE_MAX_SLOTS = 12
export const MIRROMERE_MAX_TEXT_BYTES = 1024
export const MIRROMERE_MAX_PURPOSE_BYTES = 256
export const MIRROMERE_MAX_ACCESSIBILITY_BYTES = 512
export const MIRROMERE_MAX_VECTOR_SAMPLES = 256
export const MIRROMERE_MAX_TRANSITION_MS = 2000
export const MIRROMERE_MAX_ATTENTION_BUDGET = 3

export const MIRROMERE_SCENE_IDS = [
  'ambient.idle',
  'system.starting',
  'system.degraded',
  'conversation.presence',
  'continuity.handoff-ready',
  'research.focus',
  'privacy.veil',
  'offline.local',
] as const
export const MIRROMERE_ALLOWED_INTERACTIONS = [
  'inspect_provenance',
  'continue_handoff',
  'dismiss_attention',
] as const
export const MIRROMERE_DISPLAY_ROLES = ['hud_aperture', 'native_outpost'] as const
export const MIRROMERE_SOURCE_MODES = ['runtime', 'fixture'] as const
export const MIRROMERE_PRIVACY_CLASSES = ['public_ambient', 'shared_room', 'operator_private'] as const
export const MIRROMERE_MIME_TYPES = ['image/png', 'image/jpeg', 'video/mp4', 'audio/mpeg'] as const
export const MIRROMERE_APP_VIEW_IDS = ['system_status', 'continuity', 'research_focus'] as const
export const MIRROMERE_VECTOR_FIELDS = ['vector', 'radar', 'wave'] as const
export const MIRROMERE_PRESENCE_PHASES = ['listening', 'thinking', 'responding', 'waiting'] as const
export const MIRROMERE_FRESHNESS_STATES = ['fresh', 'stale', 'unavailable'] as const
export const MIRROMERE_AVAILABILITY_STATES = ['available', 'unavailable'] as const
export const MIRROMERE_REDUCED_MOTION_MODES = ['freeze', 'simplify', 'none'] as const
export const MIRROMERE_URGENCY_STATES = ['ambient', 'normal', 'urgent'] as const
export const MIRROMERE_TRANSITION_STYLES = ['cut', 'fade', 'sweep'] as const

export type MirromereSceneId = typeof MIRROMERE_SCENE_IDS[number]
export type MirromereInteractionId = typeof MIRROMERE_ALLOWED_INTERACTIONS[number]
export type MirromerePrivacyClass = 'public_ambient' | 'shared_room' | 'operator_private'
export type MirromereVisibilityCeiling = MirromerePrivacyClass

export interface MirromereScene {
  scene_id: MirromereSceneId
  application_id: string
  application_version: string
  purpose: string
}

export type MirromereSlotContent =
  | { kind: 'status'; label: string; state: string }
  | { kind: 'text'; text: string }
  | { kind: 'media_ref'; asset_id: string; digest: string; mime_type: 'image/png' | 'image/jpeg' | 'video/mp4' | 'audio/mpeg' }
  | { kind: 'vector_field'; field: 'vector' | 'radar' | 'wave'; samples: number[] }
  | { kind: 'conversation_presence'; participant_ref: string; phase: 'listening' | 'thinking' | 'responding' | 'waiting' }
  | { kind: 'app_view'; view_id: 'system_status' | 'continuity' | 'research_focus' }

export interface MirromereSlot {
  id: string
  content: MirromereSlotContent
}

export interface MirromereEvidenceReference {
  source_id: string
  evidence_ref: string
  observed_at: string
}

export interface MirromereSurface {
  schema_version: typeof MIRROMERE_SURFACE_SCHEMA_VERSION
  surface_id: string
  outpost_id: string
  display_role: 'hud_aperture' | 'native_outpost'
  source_mode: 'runtime' | 'fixture'
  scene: MirromereScene
  slots: MirromereSlot[]
  evidence: MirromereEvidenceReference[]
  generated_at: string
  expires_at: string
  freshness: 'fresh' | 'stale' | 'unavailable'
  availability: 'available' | 'unavailable'
  privacy: {
    privacy_class: MirromerePrivacyClass
    visibility_ceiling: MirromereVisibilityCeiling
  }
  allowed_interactions: MirromereInteractionId[]
  accessibility: {
    description: string
    reduced_motion: 'freeze' | 'simplify' | 'none'
    urgency: 'ambient' | 'normal' | 'urgent'
  }
  transition: {
    style: 'cut' | 'fade' | 'sweep'
    duration_ms: number
    attention_budget: number
  }
}

const ROOT_FIELDS = [
  'schema_version', 'surface_id', 'outpost_id', 'display_role', 'source_mode', 'scene',
  'slots', 'evidence', 'generated_at', 'expires_at', 'freshness', 'availability',
  'privacy', 'allowed_interactions', 'accessibility', 'transition',
] as const
function record(value: unknown, lane: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`${lane} must be an object`)
  return value as Record<string, unknown>
}

function exactKeys(value: Record<string, unknown>, allowed: readonly string[], lane: string): void {
  for (const key of Object.keys(value)) {
    if (!allowed.includes(key)) throw new Error(`unknown ${lane} field: ${key}`)
  }
  for (const key of allowed) {
    if (!(key in value)) throw new Error(`${lane}.${key} is required`)
  }
}

function list(value: unknown, lane: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${lane} must be an array`)
  return value
}

function boundedText(value: unknown, lane: string, maxBytes: number): string {
  if (typeof value !== 'string' || value.trim() === '' || new TextEncoder().encode(value).length > maxBytes) {
    throw new Error(`${lane} must contain 1..=${maxBytes} bytes`)
  }
  return value
}

function enumValue<T extends string>(value: unknown, allowed: readonly T[], lane: string): T {
  if (typeof value !== 'string' || !allowed.includes(value as T)) throw new Error(`${lane} has an unsupported value`)
  return value as T
}

function isoDate(value: unknown, lane: string): string {
  const candidate = boundedText(value, lane, 64)
  if (Number.isNaN(Date.parse(candidate))) throw new Error(`${lane} must be an ISO timestamp`)
  return candidate
}

function integer(value: unknown, lane: string, min: number, max: number): number {
  if (!Number.isInteger(value) || (value as number) < min || (value as number) > max) {
    throw new Error(`${lane} must be an integer from ${min} through ${max}`)
  }
  return value as number
}

function rejectUnsafe(value: string): void {
  const lower = value.toLowerCase()
  if (/[<>]/.test(value) || /javascript:|https?:\/\/|rm -rf|sh -c/.test(lower)) {
    throw new Error('unsafe Mirromere content')
  }
}

function parseScene(value: unknown): MirromereScene {
  const scene = record(value, 'scene')
  exactKeys(scene, ['scene_id', 'application_id', 'application_version', 'purpose'], 'scene')
  enumValue(scene.scene_id, MIRROMERE_SCENE_IDS, 'scene.scene_id')
  rejectUnsafe(boundedText(scene.application_id, 'scene.application_id', MIRROMERE_MAX_PURPOSE_BYTES))
  boundedText(scene.application_version, 'scene.application_version', 64)
  boundedText(scene.purpose, 'scene.purpose', MIRROMERE_MAX_PURPOSE_BYTES)
  rejectUnsafe(scene.purpose as string)
  return scene as unknown as MirromereScene
}

function parseContent(value: unknown): MirromereSlotContent {
  const content = record(value, 'slot.content')
  const kind = boundedText(content.kind, 'slot.content.kind', 64)
  if (kind === 'status') {
    exactKeys(content, ['kind', 'label', 'state'], 'status content')
    rejectUnsafe(boundedText(content.label, 'status.label', 128))
    rejectUnsafe(boundedText(content.state, 'status.state', 128))
  } else if (kind === 'text') {
    exactKeys(content, ['kind', 'text'], 'text content')
    rejectUnsafe(boundedText(content.text, 'text.text', MIRROMERE_MAX_TEXT_BYTES))
  } else if (kind === 'media_ref') {
    exactKeys(content, ['kind', 'asset_id', 'digest', 'mime_type'], 'media content')
    boundedText(content.asset_id, 'media.asset_id', 128)
    const digest = boundedText(content.digest, 'media.digest', 71)
    if (!/^sha256:[A-Fa-f0-9]{64}$/.test(digest)) throw new Error('unsafe media digest')
    enumValue(content.mime_type, MIRROMERE_MIME_TYPES, 'media.mime_type')
  } else if (kind === 'vector_field') {
    exactKeys(content, ['kind', 'field', 'samples'], 'vector content')
    enumValue(content.field, MIRROMERE_VECTOR_FIELDS, 'vector.field')
    const samples = list(content.samples, 'vector.samples')
    if (samples.length === 0 || samples.length > MIRROMERE_MAX_VECTOR_SAMPLES
      || samples.some((sample) => typeof sample !== 'number' || !Number.isFinite(sample) || sample < -1 || sample > 1)) {
      throw new Error('vector samples are invalid')
    }
  } else if (kind === 'conversation_presence') {
    exactKeys(content, ['kind', 'participant_ref', 'phase'], 'presence content')
    boundedText(content.participant_ref, 'presence.participant_ref', 128)
    enumValue(content.phase, MIRROMERE_PRESENCE_PHASES, 'presence.phase')
  } else if (kind === 'app_view') {
    exactKeys(content, ['kind', 'view_id'], 'app view content')
    enumValue(content.view_id, MIRROMERE_APP_VIEW_IDS, 'app_view.view_id')
  } else {
    throw new Error(`unknown Mirromere content kind: ${kind}`)
  }
  return content as unknown as MirromereSlotContent
}

function parseSlots(value: unknown): MirromereSlot[] {
  const items = list(value, 'slots')
  if (items.length === 0 || items.length > MIRROMERE_MAX_SLOTS) throw new Error('slots exceed Mirromere bounds')
  const ids = new Set<string>()
  return items.map((item, index) => {
    const slot = record(item, `slots[${index}]`)
    exactKeys(slot, ['id', 'content'], `slots[${index}]`)
    const id = boundedText(slot.id, `slots[${index}].id`, 64)
    if (ids.has(id)) throw new Error(`duplicate slot id: ${id}`)
    ids.add(id)
    return { id, content: parseContent(slot.content) }
  })
}

function parseEvidence(value: unknown): MirromereEvidenceReference[] {
  const items = list(value, 'evidence')
  if (items.length === 0) throw new Error('Mirromere evidence is required')
  return items.map((item, index) => {
    const evidence = record(item, `evidence[${index}]`)
    exactKeys(evidence, ['source_id', 'evidence_ref', 'observed_at'], `evidence[${index}]`)
    return {
      source_id: boundedText(evidence.source_id, 'evidence.source_id', 128),
      evidence_ref: boundedText(evidence.evidence_ref, 'evidence.evidence_ref', 256),
      observed_at: isoDate(evidence.observed_at, 'evidence.observed_at'),
    }
  })
}

export function parseMirromereSurface(value: unknown, now = new Date()): MirromereSurface {
  const root = record(value, 'Mirromere surface')
  exactKeys(root, ROOT_FIELDS, 'Mirromere surface')
  if (root.schema_version !== MIRROMERE_SURFACE_SCHEMA_VERSION) throw new Error('unsupported Mirromere schema')
  rejectUnsafe(boundedText(root.surface_id, 'surface_id', 128))
  rejectUnsafe(boundedText(root.outpost_id, 'outpost_id', 128))
  enumValue(root.display_role, MIRROMERE_DISPLAY_ROLES, 'display_role')
  enumValue(root.source_mode, MIRROMERE_SOURCE_MODES, 'source_mode')
  const scene = parseScene(root.scene)
  const slots = parseSlots(root.slots)
  const evidence = parseEvidence(root.evidence)
  const generatedAt = isoDate(root.generated_at, 'generated_at')
  const expiresAt = isoDate(root.expires_at, 'expires_at')
  if (Date.parse(generatedAt) > Date.parse(expiresAt)) throw new Error('invalid Mirromere validity window')
  if (now.getTime() > Date.parse(expiresAt)) throw new Error('Mirromere surface expired')
  enumValue(root.freshness, MIRROMERE_FRESHNESS_STATES, 'freshness')
  enumValue(root.availability, MIRROMERE_AVAILABILITY_STATES, 'availability')

  const privacy = record(root.privacy, 'privacy')
  exactKeys(privacy, ['privacy_class', 'visibility_ceiling'], 'privacy')
  const privacyClass = enumValue(privacy.privacy_class, MIRROMERE_PRIVACY_CLASSES, 'privacy.privacy_class')
  const visibilityCeiling = enumValue(privacy.visibility_ceiling, MIRROMERE_PRIVACY_CLASSES, 'privacy.visibility_ceiling')
  if (MIRROMERE_PRIVACY_CLASSES.indexOf(privacyClass) > MIRROMERE_PRIVACY_CLASSES.indexOf(visibilityCeiling)) throw new Error('Mirromere privacy escalation')

  const interactionValues = list(root.allowed_interactions, 'allowed_interactions')
  const interactions = interactionValues.map((interaction) =>
    enumValue(interaction, MIRROMERE_ALLOWED_INTERACTIONS, 'interaction'))
  if (new Set(interactions).size !== interactions.length) throw new Error('duplicate Mirromere interaction')

  const accessibility = record(root.accessibility, 'accessibility')
  exactKeys(accessibility, ['description', 'reduced_motion', 'urgency'], 'accessibility')
  rejectUnsafe(boundedText(accessibility.description, 'accessibility.description', MIRROMERE_MAX_ACCESSIBILITY_BYTES))
  enumValue(accessibility.reduced_motion, MIRROMERE_REDUCED_MOTION_MODES, 'accessibility.reduced_motion')
  enumValue(accessibility.urgency, MIRROMERE_URGENCY_STATES, 'accessibility.urgency')

  const transition = record(root.transition, 'transition')
  exactKeys(transition, ['style', 'duration_ms', 'attention_budget'], 'transition')
  enumValue(transition.style, MIRROMERE_TRANSITION_STYLES, 'transition.style')
  integer(transition.duration_ms, 'transition.duration_ms', 0, MIRROMERE_MAX_TRANSITION_MS)
  integer(transition.attention_budget, 'transition.attention_budget', 0, MIRROMERE_MAX_ATTENTION_BUDGET)

  return {
    ...(root as unknown as MirromereSurface),
    scene,
    slots,
    evidence,
    allowed_interactions: interactions,
  }
}