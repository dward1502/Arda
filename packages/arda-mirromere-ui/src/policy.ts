import type {
  MirromereInteractionId,
  MirromerePrivacyClass,
  MirromereSceneId,
  MirromereSurface,
} from './contract'

export interface MirromereSceneRegistration {
  logical_id: string
  scene_ids: readonly MirromereSceneId[]
  allowed_interactions: readonly MirromereInteractionId[]
  automatic_interactions: readonly MirromereInteractionId[]
}

export const MIRROMERE_SCENE_REGISTRY: readonly MirromereSceneRegistration[] = [
  { logical_id: 'ambient.idle', scene_ids: ['ambient.idle'], allowed_interactions: ['inspect_provenance'], automatic_interactions: ['inspect_provenance'] },
  { logical_id: 'system.lifecycle', scene_ids: ['system.starting', 'system.degraded'], allowed_interactions: ['inspect_provenance', 'dismiss_attention'], automatic_interactions: ['inspect_provenance'] },
  { logical_id: 'conversation.presence', scene_ids: ['conversation.presence'], allowed_interactions: ['inspect_provenance'], automatic_interactions: ['inspect_provenance'] },
  { logical_id: 'continuity.handoff-ready', scene_ids: ['continuity.handoff-ready'], allowed_interactions: ['inspect_provenance', 'continue_handoff'], automatic_interactions: ['inspect_provenance'] },
  { logical_id: 'research.focus', scene_ids: ['research.focus'], allowed_interactions: ['inspect_provenance'], automatic_interactions: ['inspect_provenance'] },
  { logical_id: 'privacy.veil', scene_ids: ['privacy.veil'], allowed_interactions: [], automatic_interactions: [] },
  { logical_id: 'offline.local', scene_ids: ['offline.local'], allowed_interactions: [], automatic_interactions: [] },
]

export interface MirromereInteractionPolicyResult {
  accepted: boolean
  reason: string
  requires_operator_action: boolean
}

export interface MirromereInteractionReceipt {
  schema_version: 'arda.mirromere.interaction-receipt.v1'
  receipt_id: string
  surface_id: string
  scene_id: MirromereSceneId
  interaction_id: MirromereInteractionId
  requested_at: string
  recorded_at: string
  outcome: 'accepted' | 'rejected'
  status: 'requested' | 'rejected'
  requires_operator_action: boolean
  reason: string
}

export type MirromereInteractionInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>

const PRIVACY_ORDER: readonly MirromerePrivacyClass[] = [
  'public_ambient',
  'shared_room',
  'operator_private',
]

function registrationFor(sceneId: string): MirromereSceneRegistration | undefined {
  return MIRROMERE_SCENE_REGISTRY.find((entry) =>
    (entry.scene_ids as readonly string[]).includes(sceneId))
}

function requiresOperatorAction(interactionId: MirromereInteractionId): boolean {
  return interactionId === 'continue_handoff' || interactionId === 'dismiss_attention'
}

export function evaluateMirromereInteractionPolicy(
  surface: MirromereSurface,
  interactionId: MirromereInteractionId,
  explicitOperatorAction: boolean,
  now = new Date(),
): MirromereInteractionPolicyResult {
  const requires_operator_action = requiresOperatorAction(interactionId)
  const registration = registrationFor(surface.scene.scene_id)
  if (!registration) {
    return { accepted: false, reason: 'unknown_scene_id', requires_operator_action }
  }
  if (now.getTime() > Date.parse(surface.expires_at)) {
    return { accepted: false, reason: 'expired_surface', requires_operator_action }
  }
  if (PRIVACY_ORDER.indexOf(surface.privacy.privacy_class)
    > PRIVACY_ORDER.indexOf(surface.privacy.visibility_ceiling)) {
    return { accepted: false, reason: 'privacy_mismatch', requires_operator_action }
  }
  if (!surface.allowed_interactions.includes(interactionId)) {
    return { accepted: false, reason: 'interaction_not_registered_on_surface', requires_operator_action }
  }
  if (!registration.allowed_interactions.includes(interactionId)) {
    return { accepted: false, reason: 'interaction_not_registered_for_scene', requires_operator_action }
  }
  if (!explicitOperatorAction && !registration.automatic_interactions.includes(interactionId)) {
    return { accepted: false, reason: 'explicit_operator_action_required', requires_operator_action }
  }
  return { accepted: true, reason: 'request_recorded', requires_operator_action }
}

export function requestMirromereInteraction(
  surface: MirromereSurface,
  interactionId: MirromereInteractionId,
  explicitOperatorAction: boolean,
  invoke: MirromereInteractionInvoke,
  now = new Date(),
): Promise<MirromereInteractionReceipt> {
  return invoke<MirromereInteractionReceipt>('request_mirromere_interaction', {
    request: {
      surface,
      interaction_id: interactionId,
      requested_at: now.toISOString(),
      explicit_operator_action: explicitOperatorAction,
      presented_privacy_class: surface.privacy.privacy_class,
      visibility_ceiling: surface.privacy.visibility_ceiling,
    },
  })
}