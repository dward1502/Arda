import { describe, expect, it, vi } from 'vitest'
import { ambientIdleFixture } from '../../features/mirromere/fixtures'
import { requestMirromereInspection } from './BoardroomViewport'
import type { MirromereInteractionReceipt } from '../../features/mirromere/sceneRegistry'

function receipt(overrides: Partial<MirromereInteractionReceipt>): MirromereInteractionReceipt {
  return {
    schema_version: 'arda.mirromere.interaction-receipt.v1',
    receipt_id: 'receipt-test',
    surface_id: ambientIdleFixture.surface_id,
    scene_id: 'ambient.idle',
    interaction_id: 'inspect_provenance',
    requested_at: '2026-08-19T12:00:00Z',
    recorded_at: '2026-08-19T12:00:00Z',
    outcome: 'accepted',
    status: 'requested',
    requires_operator_action: false,
    reason: 'request_recorded',
    ...overrides,
  }
}

describe('Mirromere interaction wiring', () => {
  it('opens provenance only after a backend accepted requested receipt', async () => {
    const requestInteraction = vi.fn().mockResolvedValue(receipt({}))
    const openProvenance = vi.fn()

    await requestMirromereInspection(
      { ...ambientIdleFixture, source_mode: 'runtime' },
      requestInteraction,
      openProvenance,
    )

    expect(requestInteraction).toHaveBeenCalledWith(
      expect.objectContaining({ surface_id: ambientIdleFixture.surface_id }),
      'inspect_provenance',
      false,
    )
    expect(openProvenance).toHaveBeenCalledOnce()
  })

  it('does not open provenance for a rejected backend receipt', async () => {
    const requestInteraction = vi.fn().mockResolvedValue(receipt({
      outcome: 'rejected',
      status: 'rejected',
      reason: 'expired_surface',
    }))
    const openProvenance = vi.fn()

    await requestMirromereInspection(
      { ...ambientIdleFixture, source_mode: 'runtime' },
      requestInteraction,
      openProvenance,
    )

    expect(openProvenance).not.toHaveBeenCalled()
  })
})
