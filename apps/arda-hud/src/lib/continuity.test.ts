import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  createContinuityClient,
  loadContinuityProjection,
  parseContinuityProjection,
} from './continuity'

afterEach(() => vi.unstubAllGlobals())

describe('Hermes continuity projection', () => {
  it('loads an honest no-session projection', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({
      schema_version: 'arda.continuity-projection.v1',
      generated_at: '2026-08-17T20:00:00Z',
      active: false,
      session_lineage_id: null,
      current_session_id: null,
      surface_id: null,
      privacy_class: null,
      freshness: 'unavailable',
      handoff_id: null,
      handoff_state: null,
      action_ids: [],
      private_refs_withheld: false,
      topic_refs: [],
      commitment_refs: [],
      memory_scope_refs: [],
    }), { status: 200 })))
    const projection = await loadContinuityProjection('operator-1')
    expect(projection.active).toBe(false)
    expect(projection.freshness).toBe('unavailable')
  })

  it('preserves shared-surface privacy denial without synthetic references', () => {
    const projection = parseContinuityProjection({
      schema_version: 'arda.continuity-projection.v1',
      generated_at: '2026-08-17T20:00:00Z',
      active: true,
      session_lineage_id: 'lineage-1',
      current_session_id: 'session-1',
      surface_id: 'discord:shared-room',
      privacy_class: 'shared_room',
      freshness: 'fresh',
      handoff_id: 'handoff-1',
      handoff_state: 'prepared',
      action_ids: ['continue_here'],
      private_refs_withheld: true,
      topic_refs: [],
      commitment_refs: [],
      memory_scope_refs: [],
    })
    expect(projection.private_refs_withheld).toBe(true)
    expect(projection.topic_refs).toEqual([])
    expect(projection.action_ids).toEqual(['continue_here'])
  })

  it.each([
    ['phone active', true, 'discord:private-chat', 'fresh', null],
    ['desktop accepted', true, 'desktop:arda-hud', 'fresh', 'accepted'],
    ['expired handoff', false, 'discord:private-chat', 'stale', 'expired'],
  ])('parses %s source truth', (_label, active, surface, freshness, handoffState) => {
    const projection = parseContinuityProjection({
      schema_version: 'arda.continuity-projection.v1',
      generated_at: '2026-08-17T20:00:00Z',
      active,
      session_lineage_id: 'lineage-1',
      current_session_id: 'session-1',
      surface_id: surface,
      privacy_class: 'personal_device',
      freshness,
      handoff_id: handoffState ? 'handoff-1' : null,
      handoff_state: handoffState,
      action_ids: [],
      private_refs_withheld: false,
      topic_refs: ['topic:phase-2'],
      commitment_refs: ['commitment:finish-phase-2'],
      memory_scope_refs: ['vaire:scope:system-continuity'],
    })
    expect(projection.active).toBe(active)
    expect(projection.surface_id).toBe(surface)
    expect(projection.freshness).toBe(freshness)
    expect(projection.handoff_state).toBe(handoffState)
  })

  it('accepts Continue here only through the bounded handoff endpoint', async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify({
      schema_version: 'arda.surface-handoff-response.v1',
      handoff: { state: 'accepted' },
      replayed: false,
      receipt_ref: 'arda://continuity/receipts/one',
    }), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    const client = createContinuityClient('operator-1')
    await client.continueHere('handoff-1', 'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')
    expect(fetchMock).toHaveBeenCalledWith(
      'http://127.0.0.1:7878/v1/handoffs/handoff-1/accept',
      expect.objectContaining({
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-arda-operator-id': 'operator-1',
        },
        body: JSON.stringify({
          operator_ref: 'operator-1',
          idempotency_key: 'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        }),
      }),
    )
  })
})
