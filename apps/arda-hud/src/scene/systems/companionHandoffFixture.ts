// sigil: REPAIR
import type { ArdaPresenceEventInput } from '../../lib/ardaPresenceSchema'

export const COMPANION_HANDOFF_KNOWLEDGE_EVENT = {
  id: 'gate-3.5g-athena-knowledge-handoff',
  timestamp: '2026-05-18T12:10:00.000Z',
  source: 'arda',
  domain: 'knowledge',
  kind: 'presence.agent_message',
  severity: 'info',
  entity: {
    id: 'athena',
    type: 'agent',
    label: 'ATHENA',
    status: 'thinking',
  },
  message: {
    fromEntityId: 'athena',
    toEntityIds: ['arandur', 'mnemosyne'],
    channel: 'boardroom',
    summary: 'ATHENA knowledge ready',
    bodyPreview: 'Review synthesized candidate clusters before queue mutation.',
    transport: 'file_tail',
    sentiment: 'neutral',
  },
  trace: {
    agentId: 'athena',
    missionId: 'mission-arandur-capability',
    correlationId: 'gate-3.5g-companion-handoff',
  },
  tags: ['gate-3.5g', 'companion-handoff'],
} satisfies ArdaPresenceEventInput
