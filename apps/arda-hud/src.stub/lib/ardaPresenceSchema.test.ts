// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  ARDA_PRESENCE_SCHEMA_VERSION,
  normalizeArdaPresenceEvent,
  presenceRefs,
  presenceTags,
  projectArdaPresenceToHudEvent,
  type ArdaPresenceEventInput,
} from './ardaPresenceSchema'

const gate35bFixtures: ArdaPresenceEventInput[] = [
  {
    id: 'presence-agent-state-arandur',
    timestamp: '2026-05-18T12:00:00.000Z',
    source: 'arandur-presence',
    domain: 'agent',
    kind: 'presence.agent_state',
    severity: 'info',
    entity: {
      id: 'arandur',
      type: 'agent',
      label: 'Arandur',
      role: 'operator',
      status: 'active',
      missionId: 'mission-presence-contract',
      currentTask: 'Review ARDA presence contract',
    },
    scene: {
      scene: 'boardroom',
      zoneId: 'operator-ring',
      anchorId: 'hologram_anchor',
      workstationId: 'workstation-ops',
      visualHint: 'avatar_pulse',
    },
    tags: ['gate-3.5b', 'gate-3.5b'],
  },
  {
    id: 'presence-agent-message-athena',
    timestamp: 1779105601000,
    source: 'athena-review-feed',
    domain: 'knowledge',
    kind: 'presence.agent_message',
    severity: 'warn',
    entity: {
      id: 'athena',
      type: 'agent',
      label: 'ATHENA',
      status: 'thinking',
    },
    message: {
      fromEntityId: 'athena',
      toEntityIds: ['arandur'],
      channel: 'boardroom',
      summary: 'Candidate claim needs provenance review.',
      bodyPreview: 'Research claim remains review-gated until citations are verified.',
      transport: 'file_tail',
      sentiment: 'concern',
    },
    trace: {
      agentId: 'athena',
      missionId: 'mission-presence-contract',
      correlationId: 'gate-3.5b-athena-review',
    },
  },
  {
    id: 'presence-file-event-source-map',
    source: 'arda-source-map-watcher',
    domain: 'operations',
    kind: 'presence.file_event',
    severity: 'info',
    file: {
      path: 'core/state/arda_source_map.json',
      operation: 'reviewed',
      canonicality: 'candidate',
      sourceId: 'gate-3.5b-fixture',
    },
    trace: {
      taskId: 'tests-35b',
    },
  },
  {
    id: 'presence-consensus-triad',
    source: 'triad-council',
    domain: 'governance',
    kind: 'presence.consensus_state',
    severity: 'critical',
    consensus: {
      subjectId: 'mission-presence-contract',
      state: 'disputed',
      participants: ['arandur', 'athena', 'oracle'],
      summary: 'Presence runtime mutation remains blocked pending contract review.',
      triadScore: 0.82,
      resonanceScore: 0.77,
    },
    scene: {
      scene: 'boardroom',
      anchorId: 'council-table',
      visualHint: 'conflict_marker',
    },
  },
]

describe('ARDA Gate 3.5B presence fixtures', () => {
  it('normalizes the Gate 3.5B fixture set without changing schema authority', () => {
    const normalized = gate35bFixtures.map(normalizeArdaPresenceEvent)

    expect(normalized).toHaveLength(4)
    expect(normalized.map((event) => event.schemaVersion)).toEqual([
      ARDA_PRESENCE_SCHEMA_VERSION,
      ARDA_PRESENCE_SCHEMA_VERSION,
      ARDA_PRESENCE_SCHEMA_VERSION,
      ARDA_PRESENCE_SCHEMA_VERSION,
    ])
    expect(normalized.map((event) => event.kind)).toEqual([
      'presence.agent_state',
      'presence.agent_message',
      'presence.file_event',
      'presence.consensus_state',
    ])
    expect(normalized[0].tags).toEqual(['gate-3.5b'])
  })

  it('projects fixture events into read-only HUD events with refs, tags, traces, and raw provenance', () => {
    const hudEvents = gate35bFixtures.map(projectArdaPresenceToHudEvent)

    expect(hudEvents[0]).toMatchObject({
      id: 'presence-agent-state-arandur',
      schemaVersion: '1.0.0',
      domain: 'agent',
      kind: 'presence.agent_state',
      message: 'Arandur is active',
      refs: [
        { type: 'agent', id: 'arandur' },
        { type: 'mission', id: 'mission-presence-contract' },
      ],
      pinned: false,
    })
    expect(hudEvents[0].tags).toEqual(
      expect.arrayContaining([
        'gate-3.5b',
        'presence:agent_state',
        'scene:boardroom',
        'anchor:hologram_anchor',
        'workstation:workstation-ops',
        'visual:avatar_pulse',
        'entity:agent',
        'status:active',
      ]),
    )

    expect(hudEvents[1]).toMatchObject({
      domain: 'knowledge',
      message: 'Candidate claim needs provenance review.',
      trace: {
        agentId: 'athena',
        missionId: 'mission-presence-contract',
        correlationId: 'gate-3.5b-athena-review',
      },
    })
    expect(hudEvents[1].tags).toEqual(
      expect.arrayContaining(['channel:boardroom', 'transport:file_tail', 'sentiment:concern']),
    )

    expect(hudEvents[2]).toMatchObject({
      domain: 'operations',
      message: 'reviewed core/state/arda_source_map.json',
      trace: { taskId: 'tests-35b' },
    })
    expect(hudEvents[2].tags).toEqual(
      expect.arrayContaining(['file:reviewed', 'canonicality:candidate', 'presence:file_event']),
    )

    expect(hudEvents[3]).toMatchObject({
      severity: 'critical',
      pinned: true,
      domain: 'governance',
      message: 'Presence runtime mutation remains blocked pending contract review.',
      metrics: { triadScore: 0.82 },
    })
    expect(hudEvents[3].raw?.presence).toMatchObject({
      schemaVersion: ARDA_PRESENCE_SCHEMA_VERSION,
      consensus: { state: 'disputed' },
    })
    expect(hudEvents[3].raw?.adapter).toMatchObject({
      schemaVersion: ARDA_PRESENCE_SCHEMA_VERSION,
    })
  })

  it('keeps presence refs unique across entity and trace sources', () => {
    const event = normalizeArdaPresenceEvent({
      id: 'dedupe-ref-fixture',
      domain: 'agent',
      kind: 'presence.agent_state',
      entity: {
        id: 'arandur',
        type: 'agent',
        label: 'Arandur',
        missionId: 'mission-presence-contract',
      },
      trace: {
        agentId: 'arandur',
        missionId: 'mission-presence-contract',
      },
    })

    expect(presenceRefs(event)).toEqual([
      { type: 'agent', id: 'arandur' },
      { type: 'mission', id: 'mission-presence-contract' },
    ])
  })

  it('defaults unsafe or unknown input into the read-only system event envelope', () => {
    const event = normalizeArdaPresenceEvent({
      id: '35',
      domain: 'external' as never,
      kind: 'presence.runtime_mutation' as never,
      severity: 'panic' as never,
      tags: [' operator ', '', 'operator'],
    })

    expect(event).toMatchObject({
      id: '35',
      source: 'arda-presence',
      domain: 'external',
      kind: 'presence.system_event',
      severity: 'info',
      tags: ['operator'],
    })
    expect(presenceTags(event)).toEqual(expect.arrayContaining(['presence:system_event', 'operator']))
  })
})
