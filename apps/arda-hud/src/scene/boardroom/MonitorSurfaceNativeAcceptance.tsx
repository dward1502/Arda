import { useMemo, useState } from 'react'
import {
  agentClaimMonitor,
  agentClaimMonitorSurface,
  agentGetMonitorSurfaceRegistry,
  agentPushSurfacePayload,
  agentRefreshMonitorSurfaceLease,
  agentRefreshMonitorLease,
  agentReleaseMonitor,
  agentReleaseMonitorSurface,
  createMonitorSurface,
  dismissMonitorSurface,
  type AgentClaimResult,
  type BoardroomMonitorSlotSource,
} from '../../lib/boardroomSlotSettings'
import { windowManager } from '../../utils/multiWindow'
import { coerceRuntimeMonitorRegistry } from '../../lib/monitorSurfaceRegistryBridge'
import { createMonitorSessionWindowConfig } from './monitorSessionWorkstationRoute'
import type { OperatorProjection } from '../../lib/operatorProjection'
import { agentStartBrowserMonitorSession } from '../../lib/browserMonitorSession'

const SLOT_ID = 'monitor_1'
const OWNER = 'hermes-agent-acceptance'
const BINDING = 'hermes.acceptance'
const STORAGE_KEY = 'arda.monitor-surface-native-acceptance'

interface AcceptanceRecord {
  step: string
  ok: boolean
  detail: string
}

function readRecords(): AcceptanceRecord[] {
  try {
    const parsed = JSON.parse(window.sessionStorage.getItem(STORAGE_KEY) ?? '[]')
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}

function formatClaim(result: AgentClaimResult): string {
  return `${result.message}; active=${result.active}; lease=${result.leaseExpiresAtUtc || 'none'}${result.windowLabel ? `; window=${result.windowLabel}` : ''}`
}

export default function MonitorSurfaceNativeAcceptance({
  source,
  operatorProjection,
}: {
  source: BoardroomMonitorSlotSource | null
  operatorProjection: OperatorProjection | null
}) {
  const env = (import.meta as ImportMeta & { env: Record<string, string | boolean | undefined> }).env
  const enabled = env.DEV === true && env.VITE_MONITOR_ACCEPTANCE === '1'
  const [records, setRecords] = useState<AcceptanceRecord[]>(readRecords)
  const [busy, setBusy] = useState(false)
  const passed = useMemo(() => records.filter((record) => record.ok).length, [records])

  if (!enabled) return null

  const record = (step: string, ok: boolean, detail: string) => {
    setRecords((current) => {
      const next = [...current, { step, ok, detail }]
      window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(next))
      return next
    })
  }

  const run = async (step: string, action: () => Promise<{ ok: boolean; detail: string }>) => {
    setBusy(true)
    try {
      const result = await action()
      record(step, result.ok, result.detail)
    } catch (error) {
      record(step, false, error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <aside
      aria-label="Native monitor acceptance"
      style={{
        position: 'fixed',
        zIndex: 10000,
        top: '3.5rem',
        left: '1rem',
        width: '28rem',
        maxHeight: 'calc(100vh - 5rem)',
        overflow: 'auto',
        padding: '0.85rem',
        border: '1px solid #3cf6ff',
        borderRadius: '0.5rem',
        background: 'rgba(3, 10, 18, 0.96)',
        color: '#d8fbff',
        fontFamily: 'monospace',
        fontSize: '0.78rem',
      }}
    >
      <strong>Native Monitor Acceptance</strong>
      <div aria-live="polite">{passed}/{records.length} recorded steps passed</div>
      <div>UI source: active={String(source?.active ?? false)} owner={source?.claim?.owner ?? 'none'}</div>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.35rem', margin: '0.65rem 0' }}>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('claim', async () => {
            const result = await agentClaimMonitor({
              slotId: SLOT_ID,
              owner: OWNER,
              activityKind: 'agent_activity',
              payloadBinding: BINDING,
              focusMode: 'remote_preview',
              title: 'Native Acceptance Surface',
              width: 960,
              height: 640,
            })
            return { ok: result.ok && result.active, detail: formatClaim(result) }
          })}
        >
          1 Claim
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('authorized payload', async () => {
            const result = await agentPushSurfacePayload({
              slotId: SLOT_ID,
              owner: OWNER,
              payloadBinding: BINDING,
              content: 'NATIVE ACCEPTANCE PAYLOAD — authorized Hermes activity stream',
              mime: 'text/plain',
            })
            return { ok: result.ok, detail: result.message }
          })}
        >
          2 Push payload
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('create/focus', async () => {
            const result = await createMonitorSurface({
              slotId: SLOT_ID,
              sourceZoneId: 'hermes_runtime',
              focusMode: 'remote_preview',
              title: 'Native Acceptance Surface',
              width: 960,
              height: 640,
            })
            return { ok: result.ok, detail: `${result.message}; window=${result.windowLabel || 'none'}` }
          })}
        >
          3 Create/focus
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('refresh lease', async () => {
            const result = await agentRefreshMonitorLease(SLOT_ID, OWNER)
            return { ok: result.ok && result.active, detail: formatClaim(result) }
          })}
        >
          4 Refresh lease
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            window.sessionStorage.setItem('arda.monitor-surface-native-acceptance.reload-pending', '1')
            window.location.reload()
          }}
        >
          5 Reload
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('post-reload payload', async () => {
            const pending = window.sessionStorage.getItem('arda.monitor-surface-native-acceptance.reload-pending') === '1'
            const result = await agentPushSurfacePayload({
              slotId: SLOT_ID,
              owner: OWNER,
              payloadBinding: BINDING,
              content: 'POST-RELOAD PAYLOAD — persisted claim restored',
              mime: 'text/plain',
            })
            if (result.ok) window.sessionStorage.removeItem('arda.monitor-surface-native-acceptance.reload-pending')
            return { ok: pending && result.ok, detail: `${result.message}; reloadPending=${pending}` }
          })}
        >
          6 Verify reload
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('unauthorized claim rejected', async () => {
            const result = await agentClaimMonitor({
              slotId: 'monitor_2',
              owner: 'unauthorized-agent',
              activityKind: 'agent_activity',
              payloadBinding: 'hermes.acceptance',
              focusMode: 'remote_preview',
            })
            return { ok: !result.ok && !result.active, detail: formatClaim(result) }
          })}
        >
          7 Reject unauthorized
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('release/dismiss', async () => {
            const result = await agentReleaseMonitor(SLOT_ID, OWNER)
            if (result.windowLabel) await dismissMonitorSurface(result.windowLabel)
            return { ok: result.ok && !result.active, detail: formatClaim(result) }
          })}
        >
          8 Release/dismiss
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('typed same-session claim', async () => {
            const result = await agentClaimMonitorSurface({
              slotId: SLOT_ID,
              owner: { kind: 'agent', name: OWNER },
              initialContent: {
                kind: 'document',
                documentKind: 'markdown',
                source: { kind: 'local', path: 'README.md' },
              },
              ttlMs: 3_600_000,
            })
            return {
              ok: result.ok && result.session?.workstation_handoff.session_id === result.session?.surface_session_id,
              detail: `${result.message}; session=${result.session?.surface_session_id ?? 'none'}`,
            }
          })}
        >
          Phase 6 typed claim
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('typed same-session workstation', async () => {
            const registry = await agentGetMonitorSurfaceRegistry()
            const session = coerceRuntimeMonitorRegistry(registry)?.sessions[SLOT_ID]
            if (!session) return { ok: false, detail: 'No active typed monitor_1 session' }
            windowManager.open(createMonitorSessionWindowConfig(session))
            return {
              ok: session.workstation_handoff.session_id === session.surface_session_id,
              detail: `opened session=${session.surface_session_id}`,
            }
          })}
        >
          Phase 6 open workstation
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('typed same-session refresh', async () => {
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const session = registry?.sessions[SLOT_ID]
            if (!session) return { ok: false, detail: 'No active typed monitor_1 session' }
            const result = await agentRefreshMonitorSurfaceLease(
              session.surface_session_id,
              { kind: 'agent', name: OWNER },
              3_600_000,
            )
            return {
              ok: result.ok && result.session?.revision === session.revision + 1,
              detail: `${result.message}; revision=${result.session?.revision ?? 'none'}`,
            }
          })}
        >
          Phase 6 refresh session
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('typed same-session release', async () => {
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const session = registry?.sessions[SLOT_ID]
            if (!session) return { ok: false, detail: 'No active typed monitor_1 session' }
            const result = await agentReleaseMonitorSurface(
              session.surface_session_id,
              { kind: 'agent', name: OWNER },
            )
            return {
              ok: result.ok && result.session === null,
              detail: `${result.message}; released=${session.surface_session_id}`,
            }
          })}
        >
          Phase 6 release session
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('real browser monitor stream', async () => {
            const result = await agentStartBrowserMonitorSession({
              slotId: SLOT_ID,
              owner: { kind: 'agent', name: 'browser-monitor-acceptance' },
              url: 'https://time.is/',
              ttlMs: 10 * 60_000,
              captureSessionId: `browser-monitor-1-${Date.now()}`,
            })
            return {
              ok: result.capture.muted && result.capture.frameRevision >= 2 && result.claim.ok,
              detail: `${result.capture.sessionId}; pid=${result.capture.processId}; frames=${result.capture.frameRevision}; muted=${result.capture.muted}`,
            }
          })}
        >
          Start real browser monitor 1
        </button>
        <button
          type="button"
          disabled={busy || !operatorProjection}
          onClick={() => void run('P9 mounted operator projection', async () => {
            if (!operatorProjection) return { ok: false, detail: 'Canonical operator projection unavailable' }
            const result = await agentClaimMonitorSurface({
              slotId: SLOT_ID,
              owner: { kind: 'agent', name: OWNER },
              initialContent: {
                kind: 'component',
                rendererId: 'operator_projection',
                props: { ...operatorProjection },
              },
              ttlMs: 3_600_000,
            })
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const session = registry?.sessions[SLOT_ID]
            if (session) windowManager.open(createMonitorSessionWindowConfig(session))
            return {
              ok: result.ok
                && session?.content.kind === 'component'
                && session.content.rendererId === 'operator_projection',
              detail: `${result.message}; projection=${operatorProjection.projection_id}; session=${session?.surface_session_id ?? 'none'}`,
            }
          })}
        >
          P9 Mount projection
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            window.sessionStorage.removeItem(STORAGE_KEY)
            window.sessionStorage.removeItem('arda.monitor-surface-native-acceptance.reload-pending')
            setRecords([])
          }}
        >
          Clear evidence
        </button>
      </div>
      <ol style={{ margin: 0, paddingLeft: '1.4rem' }}>
        {records.map((entry, index) => (
          <li key={`${entry.step}-${index}`} style={{ color: entry.ok ? '#7fffd4' : '#ff8ba7', marginBottom: '0.3rem' }}>
            <strong>{entry.ok ? 'PASS' : 'FAIL'} {entry.step}</strong>: {entry.detail}
          </li>
        ))}
      </ol>
    </aside>
  )
}
