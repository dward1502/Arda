import { describe, expect, it } from 'vitest'
import type { ArdaBundle } from './ardaSource'
import { getCommandConsoleSurface, getReviewGateItems } from './reviewGateDerivation'

function scoutBundle(): ArdaBundle {
  return {
    queueSummary: null,
    queueFederation: null,
    flywheelPacketRuntime: null,
    l3ReadinessProjection: null,
    hermesMessages: [],
    flywheelDispatchReceipts: [],
    hermesAgentGatewayReceipts: [],
    agentConversations: [],
    scoutRequests: [
      {
        scout_request_id: 'scout-request-1',
        question: 'Which bounded sources describe geomagnetic activity?',
        requester_agent: 'athena',
        status: 'requested',
        source_policy: 'allowlisted_public_web',
        ts_utc: '2026-07-29T12:00:00Z',
      },
    ],
    scoutFindings: [
      {
        scout_finding_id: 'scout-finding-1',
        question: 'Which bounded sources describe geomagnetic activity?',
        source_agent: 'warden-scout',
        status: 'found',
        source_class: 'public_web',
        ts_utc: '2026-07-29T12:01:00Z',
      },
    ],
    scoutRuntime: {
      authority: 'advisory',
      task_promotion_allowed: false,
    },
  } as unknown as ArdaBundle
}

describe('scout projection consumer', () => {
  it('projects source policy and advisory runtime without creating review authority', () => {
    const surface = getCommandConsoleSurface(scoutBundle(), [])

    expect(surface.scoutItems).toHaveLength(2)
    expect(surface.scoutItems).toEqual(expect.arrayContaining([
      expect.objectContaining({
        id: 'scout-request-1',
        sourcePolicy: 'allowlisted_public_web',
      }),
      expect.objectContaining({
        id: 'scout-finding-1',
        sourcePolicy: 'public_web',
      }),
    ]))
    expect(surface.lanes).toContainEqual(expect.objectContaining({
      title: 'Scout',
      value: '2 records',
      detail: 'runtime projection loaded',
      status: 'partial',
    }))
    expect(surface.receipts).toEqual([])
  })
})

describe('Arandur recommendation review projection', () => {
  it('shows only recommendations still awaiting operator review', () => {
    const bundle = {
      arandurRecommendations: [
        { recommendation_id: 'pending', review_required: true, candidate: { title: 'Pending task' } },
        { recommendation_id: 'approved', review_required: false, review_status: 'approved', candidate: { title: 'Approved task' } },
      ],
      arandurMissionApprovalRequests: [],
      hadesLifecycleReviewQueue: [],
      athenaPolicyReadiness: [],
    } as unknown as ArdaBundle

    expect(getReviewGateItems(bundle, []).map((item) => item.id)).toEqual(['pending'])
  })
})