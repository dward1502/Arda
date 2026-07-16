// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { DEFAULT_AGENT_PRESENCE_STATE, deriveLatestPresenceStateFromEventLedger, presenceSupportMarkers } from './presenceState'
import { toCitadelCompanionPayload } from './companionDisplayState'

const FIXED_NOW = '2026-05-18T12:00:00.000Z'

describe('CITADEL companion display projection', () => {
  it('projects the default ARDA idle state into a textless companion payload', () => {
    expect(toCitadelCompanionPayload(DEFAULT_AGENT_PRESENCE_STATE)).toEqual({
      schema_version: 'arda.citadel.companion.v1',
      scenario: 'idle',
      phase: 'idle',
      event: 'idle',
      primary_agent: 'arandur',
      support_agents: [],
      urgency: 'normal',
      source: 'arda',
      timestamp: '1970-01-01T00:00:00.000Z',
      arda_scenario: 'idle',
    })
  })

  it('serializes ARDA camelCase fields to the snake_case CITADEL scene contract', () => {
    expect(toCitadelCompanionPayload({
      scenario: 'knowledge',
      phase: 'agent_arrival',
      primaryAgent: 'athena',
      supportAgents: ['arandur', 'mnemosyne'],
      focus: ['athena', 'mnemosyne'],
      urgency: 'normal',
      banner: 'ATHENA has new human-inbox knowledge ready for review',
      inquiry: 'Review six synthesized candidate clusters',
      action: 'Open the Gate 3.5 review packet before queue mutation',
      eventId: 'phase-6f-athena-knowledge',
      source: 'arda',
      timestamp: FIXED_NOW,
    }, FIXED_NOW)).toEqual({
      schema_version: 'arda.citadel.companion.v1',
      scenario: 'collections',
      phase: 'agent_arrival',
      event: 'inquiry',
      banner: 'ATHENA has new human-inbox knowledge read…',
      inquiry: 'Review six synthesized candidate clusters',
      action: 'Open the Gate 3.5 review packet before queue mutation',
      primary_agent: 'athena',
      support_agents: ['arandur', 'mnemosyne'],
      focus: ['athena', 'mnemosyne'],
      urgency: 'normal',
      event_id: 'phase-6f-athena-knowledge',
      source: 'arda',
      timestamp: FIXED_NOW,
      arda_scenario: 'knowledge',
    })
  })

  it('keeps high-urgency WARDEN alerts compact for the projection client', () => {
    const payload = toCitadelCompanionPayload({
      scenario: 'alert',
      phase: 'awaiting_user',
      primaryAgent: 'warden',
      supportAgents: ['charon'],
      focus: ['warden', 'charon'],
      urgency: 'high',
      banner: 'Route anomaly requires operator review before autonomous action proceeds',
      action: 'Hold routing change until approved',
      response: 'Operator acknowledgement pending',
      source: 'arda',
      timestamp: FIXED_NOW,
    })

    expect(payload.scenario).toBe('recovery')
    expect(payload.event).toBe('awaiting_user')
    expect(payload.primary_agent).toBe('warden')
    expect(payload.support_agents).toEqual(['charon'])
    expect(payload.urgency).toBe('high')
    expect(payload.banner).toHaveLength(42)
    expect(payload.banner?.endsWith('…')).toBe(true)
  })

  it('adds deterministic companion text when non-idle text fields collapse empty', () => {
    expect(toCitadelCompanionPayload({
      scenario: 'briefing',
      phase: 'agent_arrival',
      primaryAgent: 'hermes',
      supportAgents: [],
      urgency: 'normal',
      banner: '   ',
      inquiry: '',
      source: 'arda',
      timestamp: FIXED_NOW,
    }, FIXED_NOW)).toMatchObject({
      event: 'inquiry',
      banner: 'HERMES inquiry active',
      inquiry: 'Review the active request',
    })
  })

  it('projects the latest ledger-derived multi-agent presence into both HUD orbit and CITADEL payload', () => {
    const ledger = [
      JSON.stringify({
        schema: 'arda.arda.presence_event.v1',
        id: 'ignored-human-row',
        timestamp_utc: '2026-05-18T11:59:00.000Z',
        domain: 'knowledge',
        kind: 'presence.agent_state',
        entity: {
          agent: 'operator',
        },
        scene: {
          presence: {
            mode: 'advising',
            attention: 'focused',
          },
        },
      }),
      JSON.stringify({
        schema: 'arda.arda.presence_event.v1',
        id: 'gate-3.5s-ledger-smoke',
        timestamp_utc: FIXED_NOW,
        domain: 'knowledge',
        kind: 'presence.agent_state',
        entity: {
          agent: 'athena',
          mission_id: 'mission-arandur-capability',
        },
        scene: {
          presence: {
            mode: 'advising',
            attention: 'focused',
            anchor_target: 'boardroom.hologram_anchor',
            support_agents: ['arandur', 'mnemosyne', 'hermes'],
            focus_agents: ['athena'],
          },
        },
        trace: {
          correlation_id: 'gate-3.5s-ledger-smoke',
        },
      }),
    ].join('\n')

    const state = deriveLatestPresenceStateFromEventLedger(ledger, FIXED_NOW)
    expect(presenceSupportMarkers(state).map((marker) => marker.agent)).toEqual(['arandur', 'mnemosyne', 'hermes'])
    expect(toCitadelCompanionPayload(state, FIXED_NOW)).toMatchObject({
      schema_version: 'arda.citadel.companion.v1',
      scenario: 'collections',
      phase: 'agent_arrival',
      event: 'inquiry',
      primary_agent: 'athena',
      support_agents: ['arandur', 'mnemosyne', 'hermes'],
      focus: ['athena'],
      event_id: 'gate-3.5s-ledger-smoke',
      source: 'arda',
      timestamp: FIXED_NOW,
      arda_scenario: 'knowledge',
    })
  })
})
