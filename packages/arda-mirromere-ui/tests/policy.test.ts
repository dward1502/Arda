import { describe, expect, it, vi } from 'vitest'
import { ambientIdleFixture, continuityHandoffReadyFixture, systemDegradedFixture } from './fixtures'
import type { MirromereSurface } from '../src/contract'
import {
  MIRROMERE_SCENE_REGISTRY,
  evaluateMirromereInteractionPolicy,
  requestMirromereInteraction,
} from '../src/policy'

const now = new Date('2025-01-01T00:00:00.000Z')

function runtimeSurface(overrides: Partial<MirromereSurface> = {}): MirromereSurface {
  return {
    ...ambientIdleFixture,
    source_mode: 'runtime',
    generated_at: '2025-01-01T00:00:00.000Z',
    expires_at: '2025-01-01T00:00:30.000Z',
    ...overrides,
  }
}

describe('Mirromere scene registry', () => {
  it('exports exactly seven logical entries while covering the eight flattened scene ids', () => {
    expect(MIRROMERE_SCENE_REGISTRY).toHaveLength(7)
    const logicalIds = MIRROMERE_SCENE_REGISTRY.map((entry) => entry.logical_id)
    expect(logicalIds).toEqual([
      'ambient.idle',
      'system.lifecycle',
      'conversation.presence',
      'continuity.handoff-ready',
      'research.focus',
      'privacy.veil',
      'offline.local',
    ])
    const flattened = MIRROMERE_SCENE_REGISTRY.flatMap((entry) => entry.scene_ids).sort()
    expect(flattened).toEqual([
      'ambient.idle',
      'continuity.handoff-ready',
      'conversation.presence',
      'offline.local',
      'privacy.veil',
      'research.focus',
      'system.degraded',
      'system.starting',
    ])
    expect(new Set(MIRROMERE_SCENE_REGISTRY.flatMap((entry) => entry.automatic_interactions)))
      .toEqual(new Set(['inspect_provenance']))
  })

  it('allows read-only provenance inspection automatically without local success minting', async () => {
    const surface = runtimeSurface()
    expect(evaluateMirromereInteractionPolicy(surface, 'inspect_provenance', false, now)).toMatchObject({
      accepted: true,
      requires_operator_action: false,
    })
    const invoke = vi.fn().mockResolvedValue({ outcome: 'accepted', status: 'requested' })
    const receipt = await requestMirromereInteraction(surface, 'inspect_provenance', false, invoke, now)
    expect(invoke).toHaveBeenCalledWith('request_mirromere_interaction', {
      request: expect.objectContaining({
        interaction_id: 'inspect_provenance',
        requested_at: now.toISOString(),
        explicit_operator_action: false,
        presented_privacy_class: surface.privacy.privacy_class,
        visibility_ceiling: surface.privacy.visibility_ceiling,
        surface,
      }),
    })
    expect(receipt).toEqual({ outcome: 'accepted', status: 'requested' })
  })

  it('rejects unknown scenes, unregistered interactions, privacy mismatch, and expired surfaces', () => {
    expect(evaluateMirromereInteractionPolicy(
      runtimeSurface({ scene: { ...ambientIdleFixture.scene, scene_id: 'unknown.scene' as MirromereSurface['scene']['scene_id'] } }),
      'inspect_provenance',
      false,
      now,
    )).toMatchObject({ accepted: false, reason: 'unknown_scene_id' })

    expect(evaluateMirromereInteractionPolicy(
      runtimeSurface({ allowed_interactions: [] }),
      'inspect_provenance',
      false,
      now,
    )).toMatchObject({ accepted: false, reason: 'interaction_not_registered_on_surface' })

    expect(evaluateMirromereInteractionPolicy(
      runtimeSurface({ privacy: { privacy_class: 'shared_room', visibility_ceiling: 'public_ambient' } }),
      'inspect_provenance',
      false,
      now,
    )).toMatchObject({ accepted: false, reason: 'privacy_mismatch' })

    expect(evaluateMirromereInteractionPolicy(
      runtimeSurface({ expires_at: '2024-12-31T23:59:59.000Z' }),
      'inspect_provenance',
      false,
      now,
    )).toMatchObject({ accepted: false, reason: 'expired_surface' })
  })

  it('requires explicit operator action for handoff continuation and mutating dismiss attention', () => {
    expect(evaluateMirromereInteractionPolicy(
      runtimeSurface({ ...continuityHandoffReadyFixture, source_mode: 'runtime', generated_at: now.toISOString(), expires_at: '2025-01-01T00:00:30.000Z' }),
      'continue_handoff',
      false,
      now,
    )).toMatchObject({ accepted: false, reason: 'explicit_operator_action_required', requires_operator_action: true })

    expect(evaluateMirromereInteractionPolicy(
      runtimeSurface({ ...continuityHandoffReadyFixture, source_mode: 'runtime', generated_at: now.toISOString(), expires_at: '2025-01-01T00:00:30.000Z' }),
      'continue_handoff',
      true,
      now,
    )).toMatchObject({ accepted: true, requires_operator_action: true })

    expect(evaluateMirromereInteractionPolicy(
      runtimeSurface({ ...systemDegradedFixture, source_mode: 'runtime', generated_at: now.toISOString(), expires_at: '2025-01-01T00:00:30.000Z' }),
      'dismiss_attention',
      false,
      now,
    )).toMatchObject({ accepted: false, reason: 'explicit_operator_action_required', requires_operator_action: true })
  })
})