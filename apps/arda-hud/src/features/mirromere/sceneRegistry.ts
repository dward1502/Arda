import { safeTauriInvoke } from '../../lib/tauriGuard'
import {
  requestMirromereInteraction as requestSharedMirromereInteraction,
  type MirromereInteractionId,
  type MirromereInteractionInvoke,
  type MirromereInteractionReceipt,
  type MirromereSurface,
} from '@arda/mirromere-ui'

export {
  evaluateMirromereInteractionPolicy,
  MIRROMERE_SCENE_REGISTRY,
  type MirromereInteractionPolicyResult,
  type MirromereInteractionReceipt,
  type MirromereSceneRegistration,
} from '@arda/mirromere-ui'

export function requestMirromereInteraction(
  surface: MirromereSurface,
  interactionId: MirromereInteractionId,
  explicitOperatorAction: boolean,
  invoke: MirromereInteractionInvoke = safeTauriInvoke,
  now = new Date(),
): Promise<MirromereInteractionReceipt> {
  return requestSharedMirromereInteraction(
    surface,
    interactionId,
    explicitOperatorAction,
    invoke,
    now,
  )
}
