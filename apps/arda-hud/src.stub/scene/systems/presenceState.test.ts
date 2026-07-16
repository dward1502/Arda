// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  DEFAULT_AGENT_PRESENCE_STATE,
  deriveLatestPresenceStateFromEventLedger,
  derivePresenceLedgerProjection,
  derivePresenceStateFromArdaPresence,
  normalizeAgentPresenceState,
  presenceSupportMarkers,
  presenceVisualState,
} from './presenceState'

const FIXED_NOW = '2026-05-18T12:00:00.000Z'

describe('boardroom presence state', () => {
  it('defaults to a calm Arandur-centered idle state', () => {
    expect(DEFAULT_AGENT_PRESENCE_STATE).toEqual({
      scenario: 'idle',
      phase: 'idle',
      primaryAgent: 'arandur',
      supportAgents: [],
      focus: [],
      urgency: 'normal',
      source: 'arda',
      timestamp: '1970-01-01T00:00:00.000Z',
    })
  })

  it('normalizes invalid or absent inputs without mutating the default state', () => {
    const normalized = normalizeAgentPresenceState({
      scenario: 'not-a-scenario',
      phase: 'not-a-phase',
      primaryAgent: 'unknown-agent',
      supportAgents: ['athena', 'not-an-agent', 'athena'],
      focus: ['charon', 'athena', 'charon'],
      urgency: 'panic',
      banner: '  Route review  ',
      inquiry: '',
      source: 'unknown-source',
      timestamp: 'bad timestamp',
    }, FIXED_NOW)

    expect(normalized).toEqual({
      scenario: 'idle',
      phase: 'idle',
      primaryAgent: 'arandur',
      supportAgents: ['athena'],
      focus: ['charon', 'athena'],
      urgency: 'normal',
      banner: 'Route review',
      source: 'arda',
      timestamp: FIXED_NOW,
    })
    expect(DEFAULT_AGENT_PRESENCE_STATE.supportAgents).toEqual([])
    expect(DEFAULT_AGENT_PRESENCE_STATE.focus).toEqual([])
  })

  it('derives a review-gated boardroom alert from an ARDA presence event without enabling transports', () => {
    const state = derivePresenceStateFromArdaPresence({
      id: 'evt-123',
      timestamp: FIXED_NOW,
      source: 'arda',
      domain: 'security',
      kind: 'presence.agent_message',
      severity: 'warn',
      entity: {
        id: 'warden',
        type: 'agent',
        label: 'WARDEN',
        status: 'active',
      },
      message: {
        fromEntityId: 'warden',
        toEntityIds: ['arandur', 'athena', 'external-node'],
        channel: 'boardroom',
        summary: 'Review anomalous route change before action.',
        sentiment: 'concern',
        transport: 'unknown',
      },
      trace: {
        agentId: 'warden',
        taskId: 'task-7',
      },
    })

    expect(state).toMatchObject({
      scenario: 'alert',
      phase: 'awaiting_user',
      primaryAgent: 'warden',
      supportAgents: ['athena'],
      focus: ['arandur', 'athena'],
      urgency: 'high',
      banner: 'Review anomalous route change before action.',
      eventId: 'evt-123',
      source: 'arda',
      timestamp: FIXED_NOW,
    })
    expect(state).not.toHaveProperty('transport')
  })

  it('projects the latest append-only presence-event ledger row into a bounded HUD state', () => {
    const ledger = [
      '{"id":"presence-old","schema":"arda.arda.presence_event.v1","kind":"presence.agent_state","domain":"agent","timestamp_utc":"2026-05-18T10:00:00Z","entity":{"agent":"athena","mission_id":"mission-1"},"scene":{"presence":{"attention":"focused","mode":"advising","accent":"gold","anchor_target":"boardroom.hologram_anchor"}},"trace":{"correlation_id":"old"}}',
      'not-json',
      '{"id":"ignored","schema":"arda.arda.presence_event.v1","kind":"presence.system_event","domain":"system","timestamp_utc":"2026-05-18T13:00:00Z","entity":{"agent":"warden"},"scene":{"presence":{"attention":"critical","mode":"escalating","accent":"red","anchor_target":"boardroom.hologram_anchor"}}}',
      '{"id":"presence-new","schema":"arda.arda.presence_event.v1","kind":"presence.agent_state","domain":"agent","timestamp_utc":"2026-05-18T12:00:00Z","entity":{"agent":"warden","mission_id":"mission-2"},"scene":{"presence":{"attention":"elevated","mode":"escalating","accent":"amber","anchor_target":"boardroom.hologram_anchor"}},"trace":{"correlation_id":"gate-3-5-o"}}',
    ].join('\n')

    expect(deriveLatestPresenceStateFromEventLedger(ledger, FIXED_NOW)).toEqual({
      scenario: 'alert',
      phase: 'awaiting_user',
      primaryAgent: 'warden',
      supportAgents: [],
      focus: ['warden'],
      urgency: 'high',
      banner: 'WARDEN escalating with elevated attention at boardroom.hologram_anchor',
      eventId: 'presence-new',
      missionId: 'mission-2',
      anchorTarget: 'boardroom.hologram_anchor',
      source: 'arda',
      timestamp: '2026-05-18T12:00:00.000Z',
    })
  })

  it('keeps the primary agent focused while preserving selected mission and anchor cues', () => {
    const ledger = '{"id":"presence-mission-cue","schema":"arda.arda.presence_event.v1","kind":"presence.agent_state","domain":"mission","timestamp_utc":"2026-05-18T11:58:00Z","entity":{"agent":"athena","mission_id":"mission-arandur-presence-scene"},"scene":{"presence":{"attention":"focused","mode":"coordinating","accent":"cyan","anchor_target":"boardroom.view_desk_control_panel","support_agents":["hermes","charon"],"focus_agents":["hermes"]}},"trace":{"correlation_id":"gate-4-2-presence"}}'

    expect(deriveLatestPresenceStateFromEventLedger(ledger, FIXED_NOW)).toMatchObject({
      scenario: 'routing',
      phase: 'agent_arrival',
      primaryAgent: 'athena',
      supportAgents: ['hermes', 'charon'],
      focus: ['athena', 'hermes'],
      eventId: 'presence-mission-cue',
      missionId: 'mission-arandur-presence-scene',
      anchorTarget: 'boardroom.view_desk_control_panel',
    })
  })

  it('reports live presence ledger freshness metadata for the boardroom indicator', () => {
    const ledger = '{"id":"presence-fresh","schema":"arda.arda.presence_event.v1","kind":"presence.agent_state","domain":"agent","timestamp_utc":"2026-05-18T11:55:00Z","entity":{"agent":"athena","mission_id":"mission-3"},"scene":{"presence":{"attention":"focused","mode":"advising","accent":"gold","anchor_target":"boardroom.hologram_anchor"}},"trace":{"correlation_id":"fresh"}}'

    expect(derivePresenceLedgerProjection(ledger, FIXED_NOW)).toEqual({
      state: {
        scenario: 'briefing',
        phase: 'agent_arrival',
        primaryAgent: 'athena',
        supportAgents: [],
        focus: ['athena'],
        urgency: 'normal',
        banner: 'ATHENA advising with focused attention at boardroom.hologram_anchor',
        eventId: 'presence-fresh',
        missionId: 'mission-3',
        anchorTarget: 'boardroom.hologram_anchor',
        source: 'arda',
        timestamp: '2026-05-18T11:55:00.000Z',
      },
      status: {
        source: 'live_ledger',
        freshness: 'fresh',
        validEventCount: 1,
        ignoredLineCount: 0,
        malformedLineCount: 0,
        latestEventId: 'presence-fresh',
        latestTimestamp: '2026-05-18T11:55:00.000Z',
        ageSeconds: 300,
        summary: 'Live presence ledger fresh: event presence-fresh, 300s old',
      },
    })
  })

  it('reports fallback status when the presence ledger is empty or unusable', () => {
    const projection = derivePresenceLedgerProjection([
      'not-json',
      '{"id":"ignored","schema":"arda.arda.presence_event.v1","kind":"presence.system_event","timestamp_utc":"2026-05-18T11:59:00Z"}',
    ].join('\n'), FIXED_NOW)

    expect(projection.state).toEqual(DEFAULT_AGENT_PRESENCE_STATE)
    expect(projection.status).toEqual({
      source: 'fallback_default',
      freshness: 'unknown',
      validEventCount: 0,
      ignoredLineCount: 1,
      malformedLineCount: 1,
      summary: 'Fallback default: no valid agent presence rows (1 ignored, 1 malformed)',
    })
  })

  it('marks old live ledger records as stale without discarding their state', () => {
    const ledger = '{"id":"presence-stale","schema":"arda.arda.presence_event.v1","kind":"presence.agent_state","domain":"agent","timestamp_utc":"2026-05-18T11:00:00Z","entity":{"agent":"charon","mission_id":"mission-4"},"scene":{"presence":{"attention":"focused","mode":"coordinating","accent":"cyan","anchor_target":"boardroom.hologram_anchor"}},"trace":{"correlation_id":"stale"}}'
    const projection = derivePresenceLedgerProjection(ledger, FIXED_NOW)

    expect(projection.state.primaryAgent).toBe('charon')
    expect(projection.status).toMatchObject({
      source: 'live_ledger',
      freshness: 'stale',
      validEventCount: 1,
      latestEventId: 'presence-stale',
      ageSeconds: 3600,
    })
  })

  it('places focused support agents in a bounded visual orbit with stable labels and palette', () => {
    expect(presenceSupportMarkers({
      ...DEFAULT_AGENT_PRESENCE_STATE,
      supportAgents: ['athena', 'hermes', 'charon', 'warden', 'oracle', 'plutus', 'apollo'],
      focus: ['hermes', 'warden'],
    })).toEqual([
      {
        agent: 'athena',
        label: 'ATH',
        color: '#b98cff',
        angleRadians: -Math.PI / 2,
        radius: 0.87,
        phaseOffset: 0,
        isFocus: false,
      },
      {
        agent: 'hermes',
        label: 'HER',
        color: '#6ea8ff',
        angleRadians: -Math.PI / 2 + Math.PI / 3,
        radius: 0.87,
        phaseOffset: 0.85,
        isFocus: true,
      },
      {
        agent: 'charon',
        label: 'CHA',
        color: '#42f5d7',
        angleRadians: -Math.PI / 2 + (2 * Math.PI) / 3,
        radius: 0.87,
        phaseOffset: 1.7,
        isFocus: false,
      },
      {
        agent: 'warden',
        label: 'WRD',
        color: '#ffb14a',
        angleRadians: -Math.PI / 2 + Math.PI,
        radius: 0.87,
        phaseOffset: 2.55,
        isFocus: true,
      },
      {
        agent: 'oracle',
        label: 'ORC',
        color: '#ffd76a',
        angleRadians: -Math.PI / 2 + (4 * Math.PI) / 3,
        radius: 0.87,
        phaseOffset: 3.4,
        isFocus: false,
      },
      {
        agent: 'plutus',
        label: 'PLU',
        color: '#68f58f',
        angleRadians: -Math.PI / 2 + (5 * Math.PI) / 3,
        radius: 0.87,
        phaseOffset: 4.25,
        isFocus: false,
      },
    ])
  })

  it('keeps idle visual state subdued and high-urgency alerts brighter for the avatar layer', () => {
    expect(presenceVisualState(DEFAULT_AGENT_PRESENCE_STATE)).toEqual({
      pulseRate: 1.35,
      ringOpacity: 0.9,
      bodyEmissiveIntensity: 2.5,
      scanlineOpacity: 0.26,
      lightIntensity: 1.25,
      supportMarkerScale: 0.72,
    })

    expect(presenceVisualState({
      ...DEFAULT_AGENT_PRESENCE_STATE,
      scenario: 'alert',
      phase: 'awaiting_user',
      urgency: 'high',
      supportAgents: ['warden'],
    })).toEqual({
      pulseRate: 2.15,
      ringOpacity: 1,
      bodyEmissiveIntensity: 3.1,
      scanlineOpacity: 0.34,
      lightIntensity: 1.65,
      supportMarkerScale: 0.88,
    })
  })
})
