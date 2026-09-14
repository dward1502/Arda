import type {
  AgentSurfaceOwner,
  MonitorContentDescriptor,
  MonitorSessionRegistryDescriptor,
  MonitorSurfaceClaimRequest,
  UpperMonitorSlotId,
} from '../../lib/monitorSurfaceContract'
import type { OperatorProjection } from '../../lib/operatorProjection'

export interface NativeAcceptanceSession {
  slotId: UpperMonitorSlotId
  owner: AgentSurfaceOwner
  content: MonitorContentDescriptor
}

const ACCEPTANCE_TTL_MS = 3_600_000

export function createNativeAcceptanceSessions(
  operatorProjection: OperatorProjection,
): NativeAcceptanceSession[] {
  return [
    {
      slotId: 'monitor_1',
      owner: { kind: 'agent', name: 'acceptance-document' },
      content: {
        kind: 'document',
        documentKind: 'markdown',
        source: { kind: 'local', path: 'README.md' },
      },
    },
    {
      slotId: 'monitor_2',
      owner: { kind: 'agent', name: 'acceptance-image' },
      content: {
        kind: 'image',
        source: {
          kind: 'local',
          path: 'apps/arda-hud/src/assets/scene/world/upper_monitor_1/upper_monitor_1_preview.png',
        },
        fit: 'contain',
        alt: 'ARDA upper-monitor acceptance image',
      },
    },
    {
      slotId: 'monitor_3',
      owner: { kind: 'agent', name: 'acceptance-video' },
      content: {
        kind: 'video',
        source: {
          kind: 'remote',
          url: 'https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4',
        },
        fit: 'contain',
        loop: true,
        autoplay: true,
        muted: true,
      },
    },
    {
      slotId: 'monitor_4',
      owner: { kind: 'agent', name: 'acceptance-terminal' },
      content: {
        kind: 'terminal',
        sessionId: 'native-acceptance-readonly',
        readOnly: true,
        theme: 'arda-command',
      },
    },
    {
      slotId: 'monitor_5',
      owner: { kind: 'agent', name: 'acceptance-projection' },
      content: {
        kind: 'component',
        rendererId: 'operator_projection',
        props: { ...operatorProjection },
      },
    },
  ]
}

export function toClaimRequest(session: NativeAcceptanceSession): MonitorSurfaceClaimRequest {
  return {
    slotId: session.slotId,
    owner: session.owner,
    initialContent: session.content,
    ttlMs: ACCEPTANCE_TTL_MS,
  }
}

export function serializeAcceptanceOwner(owner: AgentSurfaceOwner): string {
  if (owner.kind === 'operator') return `operator:${owner.id}`
  return `${owner.kind}:${owner.name}`
}

export function verifyNativeAcceptanceRegistry(
  registry: MonitorSessionRegistryDescriptor,
  sessions: NativeAcceptanceSession[],
): { ok: boolean; detail: string } {
  const failures = sessions.flatMap((expected) => {
    const record = registry.sessions[expected.slotId]
    if (!record) return [`${expected.slotId}:missing`]
    if (record.owner !== serializeAcceptanceOwner(expected.owner)) {
      return [`${expected.slotId}:owner=${record.owner}`]
    }
    if (record.content.kind !== expected.content.kind) {
      return [`${expected.slotId}:kind=${record.content.kind}`]
    }
    if (record.workstation_handoff.mode !== 'same_live_session'
      || record.workstation_handoff.session_id !== record.surface_session_id) {
      return [`${expected.slotId}:handoff-mismatch`]
    }
    return []
  })
  return {
    ok: failures.length === 0 && Object.keys(registry.sessions).length === sessions.length,
    detail: failures.length === 0
      ? `sessions=${sessions.length}; owners=${new Set(sessions.map(({ owner }) => serializeAcceptanceOwner(owner))).size}; handoffs=same_live_session`
      : failures.join(', '),
  }
}
