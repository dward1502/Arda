import { useMemo, useState } from 'react'
import {
  agentClaimMonitor,
  agentPushSurfacePayload,
  agentRefreshMonitorLease,
  agentReleaseMonitor,
  createMonitorSurface,
  dismissMonitorSurface,
  type AgentClaimResult,
  type BoardroomMonitorSlotSource,
} from '../../lib/boardroomSlotSettings'

const SLOT_ID = 'monitor_left_1'
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

export default function MonitorSurfaceNativeAcceptance({ source }: { source: BoardroomMonitorSlotSource | null }) {
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
              slotId: 'monitor_left_2',
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
