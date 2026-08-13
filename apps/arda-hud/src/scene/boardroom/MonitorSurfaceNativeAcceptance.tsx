import { useEffect, useMemo, useRef, useState } from 'react'
import {
  agentClaimMonitor,
  agentClaimMonitorSurface,
  agentGetMonitorSurfaceRegistry,
  agentPatchMonitorSurfacePlayback,
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
import {
  agentClickBrowserMonitorSession,
  agentGetBrowserMonitorSession,
  agentNavigateBrowserMonitorSession,
  agentStartBrowserMonitorSession,
  agentStopBrowserMonitorSession,
  type BrowserMonitorSessionRequest,
} from '../../lib/browserMonitorSession'
import {
  getPtyMonitorSession,
  startPtyMonitorSession,
  stopPtyMonitorSession,
  writePtyMonitorSession,
} from '../../lib/ptyMonitorSession'
import type { AgentSurfaceOwner } from '../../lib/monitorSurfaceContract'

const SLOT_ID = 'monitor_1'
const SECOND_SLOT_ID = 'monitor_2'
const OWNER = 'hermes-agent-acceptance'
const BINDING = 'hermes.acceptance'
const PTY_OWNER = 'arda.agent.hermes-acceptance'
const PTY_SESSION_ID = 'arda-p9-live-pty'
const DOGFOOD_PTY_OWNER = 'arda.agent.p9-dogfood-terminal'
const DOGFOOD_PTY_SESSION_ID = 'arda-p9-dogfood-terminal'
const STORAGE_KEY = 'arda.monitor-surface-native-acceptance'
const WALKTHROUGH_OWNER_PREFIX = `${OWNER}-p9-`
const WALKTHROUGH_SLOTS = ['monitor_1', 'monitor_2', 'monitor_3', 'monitor_4', 'monitor_5'] as const
const ACCEPTANCE_OPERATOR_PROJECTION: OperatorProjection = {
  schema_version: 'arda.operator-projection.v1',
  projection_id: 'monitor-native-acceptance-projection',
  generated_at: '2026-08-12T00:00:00.000Z',
  authority: 'read_only',
  freshness: 'fresh',
  objectives: [], runs: [], capabilities: [], pending_approvals: [], councils: [],
  personal_operations: { captures: 0, resumable_items: 0, reminders: [] },
  joulework: { budget_joules: 1, consumed_joules: 0, remaining_joules: 1, source: 'synthetic_restoration', source_confidence: 1 },
  evidence: [], communications: [], dependencies: [],
}

interface AcceptanceRecord {
  step: string
  ok: boolean
  detail: string
}

interface ActiveBrowserCapture {
  sessionId: string
  owner: string
  surfaceSessionId: string
  surfaceOwner: AgentSurfaceOwner
}

async function stopBrowserCapture(active: ActiveBrowserCapture): Promise<void> {
  await agentStopBrowserMonitorSession({ sessionId: active.sessionId, owner: active.owner })
  const released = await agentReleaseMonitorSurface(active.surfaceSessionId, active.surfaceOwner)
  if (!released.ok) throw new Error(released.message || `failed to release ${active.surfaceSessionId}`)
}

async function releaseStaleAcceptanceClaim(slotId: string): Promise<void> {
  const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
  const session = registry?.sessions[slotId]
  if (!session) return
  const acceptanceOwner = session.owner === `agent:${OWNER}`
    || session.owner.startsWith(`agent:${WALKTHROUGH_OWNER_PREFIX}`)
    || session.owner === 'agent:browser-monitor-acceptance'
    || session.owner.startsWith(`agent:${OWNER}-browser-`)
  if (!acceptanceOwner) return
  const released = await agentReleaseMonitorSurface(
    session.surface_session_id,
    { kind: 'agent', name: session.owner.slice('agent:'.length) },
  )
  if (!released.ok) throw new Error(released.message || `failed to reclaim ${session.surface_session_id}`)
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
  const acceptanceProjection = operatorProjection ?? ACCEPTANCE_OPERATOR_PROJECTION
  const [records, setRecords] = useState<AcceptanceRecord[]>(readRecords)
  const [busy, setBusy] = useState(false)
  const activeBrowserCapturesRef = useRef<ActiveBrowserCapture[]>([])
  const passed = useMemo(() => records.filter((record) => record.ok).length, [records])

  useEffect(() => () => {
    const activeCaptures = activeBrowserCapturesRef.current.splice(0)
    if (activeCaptures.length > 0) {
      void Promise.allSettled(activeCaptures.map(stopBrowserCapture))
    }
  }, [])

  if (!enabled) return null

  const claimWalkthroughSurface = async (
    slotId: typeof WALKTHROUGH_SLOTS[number],
    ownerName: string,
    initialContent: Parameters<typeof agentClaimMonitorSurface>[0]['initialContent'],
    ttlMs = 10 * 60_000,
  ) => agentClaimMonitorSurface({
    slotId,
    owner: { kind: 'agent', name: ownerName },
    initialContent,
    ttlMs,
  })

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
            const priorCaptures = activeBrowserCapturesRef.current.splice(0)
            await Promise.allSettled(priorCaptures.map(stopBrowserCapture))
            await releaseStaleAcceptanceClaim(SLOT_ID)
            const surfaceOwner: AgentSurfaceOwner = { kind: 'agent', name: 'browser-monitor-acceptance' }
            const result = await agentStartBrowserMonitorSession({
              slotId: SLOT_ID,
              owner: surfaceOwner,
              url: 'https://www.youtube.com/watch?v=YE7VzlLtp-4',
              ttlMs: 10 * 60_000,
              captureSessionId: `browser-monitor-1-${Date.now()}`,
            })
            activeBrowserCapturesRef.current.push({
              sessionId: result.capture.sessionId,
              owner: result.capture.owner,
              surfaceSessionId: result.claim.session!.surface_session_id,
              surfaceOwner,
            })
            return {
              ok: result.capture.muted && result.capture.frameRevision >= 2 && result.claim.ok,
              detail: `${result.capture.sessionId}; pid=${result.capture.processId}; frames=${result.capture.frameRevision}; muted=${result.capture.muted}`,
            }
          })}
        >
          start browser
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 five native content sessions', async () => {
            const priorCaptures = activeBrowserCapturesRef.current.splice(0)
            await Promise.allSettled(priorCaptures.map(stopBrowserCapture))
            await Promise.all(WALKTHROUGH_SLOTS.map(releaseStaleAcceptanceClaim))

            const browserOwner: AgentSurfaceOwner = { kind: 'agent', name: `${WALKTHROUGH_OWNER_PREFIX}youtube` }
            const browser = await agentStartBrowserMonitorSession({
              slotId: 'monitor_1',
              owner: browserOwner,
              url: 'https://www.youtube.com/watch?v=YE7VzlLtp-4',
              ttlMs: 10 * 60_000,
              captureSessionId: `p9-youtube-${Date.now()}`,
            })
            activeBrowserCapturesRef.current.push({
              sessionId: browser.capture.sessionId,
              owner: browser.capture.owner,
              surfaceSessionId: browser.claim.session!.surface_session_id,
              surfaceOwner: browserOwner,
            })

            const results = await Promise.all([
              claimWalkthroughSurface('monitor_2', `${WALKTHROUGH_OWNER_PREFIX}video`, {
                kind: 'video',
                source: { kind: 'remote', url: 'https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4' },
                mime: 'video/mp4', fit: 'cover', loop: true, autoplay: true, muted: true,
              }),
              claimWalkthroughSurface('monitor_3', `${WALKTHROUGH_OWNER_PREFIX}image`, {
                kind: 'image',
                source: { kind: 'local', path: 'apps/arda-hud/src/assets/scene/world/boardroom_physical_stage/boardroom_physical_stage_reference.png' },
                fit: 'contain', alt: 'ARDA boardroom physical stage reference',
              }),
              claimWalkthroughSurface('monitor_4', `${WALKTHROUGH_OWNER_PREFIX}document`, {
                kind: 'document', documentKind: 'markdown', source: { kind: 'local', path: 'README.md' },
              }),
              claimWalkthroughSurface('monitor_5', `${WALKTHROUGH_OWNER_PREFIX}projection`, {
                kind: 'component', rendererId: 'operator_projection', props: { ...acceptanceProjection },
              }),
            ])
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const sessions = WALKTHROUGH_SLOTS.map((slotId) => registry?.sessions[slotId]).filter(Boolean)
            const owners = new Set(sessions.map((session) => session!.owner))
            const kinds = sessions.map((session) => session!.content.kind).join(',')
            return {
              ok: browser.claim.ok && browser.capture.muted && browser.capture.frameRevision >= 2
                && results.every((result) => result.ok) && sessions.length === 5 && owners.size === 5,
              detail: `sessions=${sessions.length}; owners=${owners.size}; kinds=${kinds}; youtubeFrames=${browser.capture.frameRevision}`,
            }
          })}
        >
          P9 Claim five content sessions
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 open all same-session workstations', async () => {
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const sessions = WALKTHROUGH_SLOTS.map((slotId) => registry?.sessions[slotId]).filter(Boolean)
            sessions.forEach((session) => windowManager.open(createMonitorSessionWindowConfig(session!)))
            const exact = sessions.every((session) =>
              session!.workstation_handoff.mode === 'same_live_session'
              && session!.workstation_handoff.session_id === session!.surface_session_id)
            return { ok: sessions.length === 5 && exact, detail: `opened=${sessions.length}; exactSameSession=${exact}` }
          })}
        >
          P9 Open all workstations
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 isolated update release expiry reassignment', async () => {
            let registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const video = registry?.sessions.monitor_2
            const image = registry?.sessions.monitor_3
            const document = registry?.sessions.monitor_4
            if (!video || !image || !document) return { ok: false, detail: 'five-session walkthrough is not active' }
            const survivorIds = ['monitor_1', 'monitor_2', 'monitor_4', 'monitor_5']
              .map((slotId) => registry?.sessions[slotId]?.surface_session_id)
            const patched = await agentPatchMonitorSurfacePlayback(
              video.surface_session_id,
              { kind: 'agent', name: `${WALKTHROUGH_OWNER_PREFIX}video` },
              video.revision,
              { playing: true, currentTime: 2, volume: 0 },
            )
            const released = await agentReleaseMonitorSurface(
              image.surface_session_id,
              { kind: 'agent', name: `${WALKTHROUGH_OWNER_PREFIX}image` },
            )
            registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const isolated = survivorIds.every((id, index) =>
              registry?.sessions[['monitor_1', 'monitor_2', 'monitor_4', 'monitor_5'][index]]?.surface_session_id === id)
            const reassigned = await claimWalkthroughSurface('monitor_3', `${WALKTHROUGH_OWNER_PREFIX}sixth-agent`, {
              kind: 'image',
              source: { kind: 'local', path: 'apps/arda-hud/src/scene/boardroom/__visual_baselines__/boardroom-default-web.png' },
              fit: 'cover', alt: 'Reassigned boardroom baseline',
            })
            const shortened = await agentRefreshMonitorSurfaceLease(
              document.surface_session_id,
              { kind: 'agent', name: `${WALKTHROUGH_OWNER_PREFIX}document` },
              1_000,
            )
            await new Promise((resolve) => window.setTimeout(resolve, 1_200))
            const afterExpiry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const expired = new Date(afterExpiry!.sessions.monitor_4.lease_expires_at_utc).getTime() <= Date.now()
            return {
              ok: patched.ok && patched.session?.revision === video.revision + 1 && released.ok
                && isolated && reassigned.ok && shortened.ok && expired,
              detail: `update=${video.revision}->${patched.session?.revision}; releaseIsolated=${isolated}; reassigned=${reassigned.session?.owner}; expiredOnly=monitor_4:${expired}`,
            }
          })}
        >
          P9 Lifecycle isolation
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 frame-blocked capture path', async () => {
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const session = registry?.sessions.monitor_1
            const capture = activeBrowserCapturesRef.current.find((item) => item.surfaceSessionId === session?.surface_session_id)
            return {
              ok: session?.content.kind === 'remote_session' && session.content.transport === 'mjpeg' && Boolean(capture),
              detail: session?.content.kind === 'remote_session'
                ? `real Chromium/CDP capture; transport=${session.content.transport}; session=${session.content.sessionId}`
                : 'no real capture-backed browser session is active',
            }
          })}
        >
          P9 Verify blocked-frame path
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 two-browser trailer navigation isolation', async () => {
            const priorCaptures = activeBrowserCapturesRef.current.splice(0)
            await Promise.allSettled(priorCaptures.map(stopBrowserCapture))
            await Promise.all([releaseStaleAcceptanceClaim(SLOT_ID), releaseStaleAcceptanceClaim(SECOND_SLOT_ID)])
            const startedAt = Date.now()
            const requests: BrowserMonitorSessionRequest[] = [
              {
                slotId: SLOT_ID,
                owner: { kind: 'agent' as const, name: `${OWNER}-browser-a` },
                url: 'https://www.youtube.com/watch?v=QeCItSg-wmI',
                ttlMs: 120_000,
                captureSessionId: `browser-monitor-a-${startedAt}`,
              },
              {
                slotId: SECOND_SLOT_ID,
                owner: { kind: 'agent' as const, name: `${OWNER}-browser-b` },
                url: 'https://threejs.org/examples/webgl_geometry_cube.html',
                ttlMs: 180_000,
                captureSessionId: `browser-monitor-b-${startedAt}`,
              },
            ]
            const results = await Promise.allSettled(requests.map(agentStartBrowserMonitorSession))
            const activeResults = results.flatMap((result) => result.status === 'fulfilled' ? [result.value] : [])
            activeBrowserCapturesRef.current.push(...activeResults.map(({ capture, claim }, index) => ({
              sessionId: capture.sessionId,
              owner: capture.owner,
              surfaceSessionId: claim.session!.surface_session_id,
              surfaceOwner: requests[index].owner,
            })))
            const failure = results.find((result) => result.status === 'rejected')
            if (failure?.status === 'rejected') {
              const rollback = activeBrowserCapturesRef.current.filter(
                (active) => activeResults.some(({ capture }) => capture.sessionId === active.sessionId),
              )
              await Promise.allSettled(rollback.map(stopBrowserCapture))
              activeBrowserCapturesRef.current = activeBrowserCapturesRef.current.filter(
                (active) => !activeResults.some(({ capture }) => capture.sessionId === active.sessionId),
              )
              throw failure.reason
            }
            const [trailerStarted, navigatorStarted] = activeResults
            const trailerBeforeNavigation = await agentGetBrowserMonitorSession(trailerStarted.capture.sessionId)
            const navigatorAfterNavigation = await agentNavigateBrowserMonitorSession(
              navigatorStarted.capture,
              'https://threejs.org/examples/webgl_animation_keyframes.html',
            )
            let crossSessionPointerRejected = false
            try {
              await agentClickBrowserMonitorSession(
                { ...trailerStarted.capture, owner: navigatorStarted.capture.owner },
                { x: 640, y: 360 },
              )
            } catch {
              crossSessionPointerRejected = true
            }
            const trailerAfterNavigation = await agentGetBrowserMonitorSession(trailerStarted.capture.sessionId)
            const navigatorObserved = await agentGetBrowserMonitorSession(navigatorStarted.capture.sessionId)
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const trailerSurface = registry?.sessions[SLOT_ID]
            const navigatorSurface = registry?.sessions[SECOND_SLOT_ID]
            if (trailerSurface) windowManager.open(createMonitorSessionWindowConfig(trailerSurface))
            if (navigatorSurface) windowManager.open(createMonitorSessionWindowConfig(navigatorSurface))
            const distinct = trailerAfterNavigation.processId !== navigatorObserved.processId
              && trailerAfterNavigation.sessionId !== navigatorObserved.sessionId
              && trailerAfterNavigation.owner !== navigatorObserved.owner
              && trailerAfterNavigation.streamUrl !== navigatorObserved.streamUrl
              && trailerSurface?.surface_session_id !== navigatorSurface?.surface_session_id
              && trailerSurface?.lease_expires_at_utc !== navigatorSurface?.lease_expires_at_utc
            const isolatedNavigation = trailerAfterNavigation.url === trailerBeforeNavigation.url
              && trailerAfterNavigation.revision === trailerBeforeNavigation.revision
              && navigatorObserved.url === navigatorAfterNavigation.url
              && navigatorObserved.revision === navigatorStarted.capture.revision + 1
            return {
              ok: activeResults.length === 2 && distinct && isolatedNavigation && crossSessionPointerRejected
                && trailerAfterNavigation.muted && navigatorObserved.muted
                && trailerAfterNavigation.frameRevision >= 2 && navigatorObserved.frameRevision >= 2,
              detail: `trailer=${trailerAfterNavigation.sessionId}; pid=${trailerAfterNavigation.processId}; rev=${trailerAfterNavigation.revision}; frames=${trailerAfterNavigation.frameRevision}; muted=${trailerAfterNavigation.muted}; lease=${trailerSurface?.lease_expires_at_utc} | navigator=${navigatorObserved.sessionId}; pid=${navigatorObserved.processId}; rev=${navigatorObserved.revision}; frames=${navigatorObserved.frameRevision}; muted=${navigatorObserved.muted}; lease=${navigatorSurface?.lease_expires_at_utc}; navigated=${navigatorObserved.url}; distinct=${distinct}; isolated=${isolatedNavigation}; crossPointerRejected=${crossSessionPointerRejected}; audioPolicy=per-process-mute`,
            }
          })}
        >
          verify 2 browsers + isolation
        </button>
        <button
          type="button"
          disabled={busy || activeBrowserCapturesRef.current.length === 0}
          onClick={() => void run('stop-browsers', async () => {
            const activeCaptures = activeBrowserCapturesRef.current.splice(0)
            const results = await Promise.allSettled(activeCaptures.map(stopBrowserCapture))
            const failed = results.filter((result) => result.status === 'rejected')
            return {
              ok: failed.length === 0,
              detail: `stopped=${results.length - failed.length}; failed=${failed.length}`,
            }
          })}
        >
          stop browsers
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 live PTY session', async () => {
            await releaseStaleAcceptanceClaim(SLOT_ID)
            await startPtyMonitorSession(
              PTY_SESSION_ID,
              PTY_OWNER,
              "printf 'ARDA_PTY_READY\\n'; while IFS= read -r line; do printf 'ARDA_PTY:%s\\n' \"$line\"; done",
            )
            let ready = await getPtyMonitorSession(PTY_SESSION_ID)
            for (let attempt = 0; attempt < 30 && !ready.output.includes('ARDA_PTY_READY'); attempt += 1) {
              await new Promise((resolve) => window.setTimeout(resolve, 40))
              ready = await getPtyMonitorSession(PTY_SESSION_ID)
            }
            let wrongOwnerRejected = false
            try {
              await writePtyMonitorSession(PTY_SESSION_ID, 'arda.agent.other', ready.revision, 'wrong-owner\n')
            } catch {
              wrongOwnerRejected = true
            }
            const claimed = await agentClaimMonitorSurface({
              slotId: SLOT_ID,
              owner: { kind: 'agent', name: OWNER },
              initialContent: { kind: 'terminal', sessionId: PTY_SESSION_ID, readOnly: false, theme: 'arda-cyan' },
              ttlMs: 3_600_000,
            })
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const surface = registry?.sessions[SLOT_ID]
            if (surface) windowManager.open(createMonitorSessionWindowConfig(surface))
            const handedOff = await writePtyMonitorSession(
              PTY_SESSION_ID,
              PTY_OWNER,
              ready.revision,
              'workstation-intervention\n',
            )
            let observed = handedOff
            for (let attempt = 0; attempt < 30 && !observed.output.includes('ARDA_PTY:workstation-intervention'); attempt += 1) {
              await new Promise((resolve) => window.setTimeout(resolve, 40))
              observed = await getPtyMonitorSession(PTY_SESSION_ID)
            }
            return {
              ok: claimed.ok
                && wrongOwnerRejected
                && surface?.content.kind === 'terminal'
                && surface.content.sessionId === PTY_SESSION_ID
                && observed.output.includes('ARDA_PTY:workstation-intervention')
                && observed.outputRevision > ready.outputRevision,
              detail: `session=${observed.sessionId}; owner=${observed.owner}; pid=${observed.processId}; rev=${observed.revision}; streamRev=${ready.outputRevision}->${observed.outputRevision}; wrongOwnerRejected=${wrongOwnerRejected}; handoff=${surface?.workstation_handoff.session_id ?? 'none'}`,
            }
          })}
        >
          P9 live PTY + handoff
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 stop PTY', async () => {
            await stopPtyMonitorSession(PTY_SESSION_ID, PTY_OWNER)
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const surface = registry?.sessions[SLOT_ID]
            const released = surface?.content.kind === 'terminal' && surface.content.sessionId === PTY_SESSION_ID
              ? await agentReleaseMonitorSurface(surface.surface_session_id, { kind: 'agent', name: OWNER })
              : null
            let unavailable = false
            try { await getPtyMonitorSession(PTY_SESSION_ID) } catch { unavailable = true }
            return {
              ok: unavailable && (released == null || released.ok),
              detail: `session=${PTY_SESSION_ID}; unavailable=${unavailable}; surfaceReleased=${released?.ok ?? 'not-active'}`,
            }
          })}
        >
          stop PTY
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 three-surface dogfood', async () => {
            const priorCaptures = activeBrowserCapturesRef.current.splice(0)
            await Promise.allSettled(priorCaptures.map(stopBrowserCapture))
            await Promise.all(WALKTHROUGH_SLOTS.slice(0, 3).map(releaseStaleAcceptanceClaim))
            const research = await claimWalkthroughSurface('monitor_1', `${WALKTHROUGH_OWNER_PREFIX}research`, {
              kind: 'document',
              documentKind: 'markdown',
              source: { kind: 'local', path: 'docs/plans/2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md' },
            }, 180_000)
            const browserRequest: BrowserMonitorSessionRequest = {
              slotId: 'monitor_2',
              owner: { kind: 'agent', name: `${WALKTHROUGH_OWNER_PREFIX}browser-operation` },
              url: 'https://docs.rs/portable-pty/latest/portable_pty/',
              ttlMs: 180_000,
              captureSessionId: `p9-dogfood-browser-${Date.now()}`,
            }
            const browser = await agentStartBrowserMonitorSession(browserRequest)
            activeBrowserCapturesRef.current.push({
              sessionId: browser.capture.sessionId,
              owner: browser.capture.owner,
              surfaceSessionId: browser.claim.session!.surface_session_id,
              surfaceOwner: browserRequest.owner,
            })
            await startPtyMonitorSession(
              DOGFOOD_PTY_SESSION_ID,
              DOGFOOD_PTY_OWNER,
              "cargo test --manifest-path src-tauri/Cargo.toml pty_capture --lib --quiet; printf 'ARDA_BUILD_OBSERVED\\n'; while IFS= read -r line; do printf 'ARDA_INTERVENTION:%s\\n' \"$line\"; done",
            )
            const terminal = await claimWalkthroughSurface('monitor_3', `${WALKTHROUGH_OWNER_PREFIX}terminal-build`, {
              kind: 'terminal', sessionId: DOGFOOD_PTY_SESSION_ID, readOnly: false, theme: 'arda-cyan',
            }, 180_000)
            let build = await getPtyMonitorSession(DOGFOOD_PTY_SESSION_ID)
            for (let attempt = 0; attempt < 300 && !build.output.includes('ARDA_BUILD_OBSERVED'); attempt += 1) {
              await new Promise((resolve) => window.setTimeout(resolve, 100))
              build = await getPtyMonitorSession(DOGFOOD_PTY_SESSION_ID)
            }
            await writePtyMonitorSession(
              DOGFOOD_PTY_SESSION_ID,
              DOGFOOD_PTY_OWNER,
              build.revision,
              'exact-session-workstation\n',
            )
            let intervened = build
            for (let attempt = 0; attempt < 30 && !intervened.output.includes('ARDA_INTERVENTION:exact-session-workstation'); attempt += 1) {
              await new Promise((resolve) => window.setTimeout(resolve, 40))
              intervened = await getPtyMonitorSession(DOGFOOD_PTY_SESSION_ID)
            }
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const sessions = WALKTHROUGH_SLOTS.slice(0, 3).map((slotId) => registry?.sessions[slotId])
            sessions.forEach((session) => { if (session) windowManager.open(createMonitorSessionWindowConfig(session)) })
            const exactHandoffs = sessions.every((session) => session?.workstation_handoff.mode === 'same_live_session'
              && session.workstation_handoff.session_id === session.surface_session_id)
            return {
              ok: research.ok && terminal.ok
                && browser.capture.frameRevision >= 1
                && intervened.output.includes('ARDA_BUILD_OBSERVED')
                && intervened.output.includes('ARDA_INTERVENTION:exact-session-workstation')
                && exactHandoffs,
              detail: `research=${research.session?.surface_session_id}; browser=${browser.capture.sessionId}/frames=${browser.capture.frameRevision}; terminal=${terminal.session?.surface_session_id}/streamRev=${intervened.outputRevision}; exactHandoffs=${exactHandoffs}; lowerApproval=operator-observation-required`,
            }
          })}
        >
          P9 dogfood 3 surfaces
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 stop dogfood', async () => {
            const captures = activeBrowserCapturesRef.current.splice(0)
            const browserStops = await Promise.allSettled(captures.map(stopBrowserCapture))
            await stopPtyMonitorSession(DOGFOOD_PTY_SESSION_ID, DOGFOOD_PTY_OWNER).catch(() => undefined)
            const registry = coerceRuntimeMonitorRegistry(await agentGetMonitorSurfaceRegistry())
            const releases = await Promise.all(WALKTHROUGH_SLOTS.slice(0, 3).map(async (slotId) => {
              const session = registry?.sessions[slotId]
              if (!session || !session.owner.startsWith(`agent:${WALKTHROUGH_OWNER_PREFIX}`)) return true
              const result = await agentReleaseMonitorSurface(
                session.surface_session_id,
                { kind: 'agent', name: session.owner.slice('agent:'.length) },
              )
              return result.ok
            }))
            let ptyUnavailable = false
            try { await getPtyMonitorSession(DOGFOOD_PTY_SESSION_ID) } catch { ptyUnavailable = true }
            return {
              ok: browserStops.every((result) => result.status === 'fulfilled') && releases.every(Boolean) && ptyUnavailable,
              detail: `browserStopped=${browserStops.length}; ptyUnavailable=${ptyUnavailable}; surfacesReleased=${releases.filter(Boolean).length}/3`,
            }
          })}
        >
          stop dogfood
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run('P9 mounted operator projection', async () => {
            const result = await agentClaimMonitorSurface({
              slotId: SLOT_ID,
              owner: { kind: 'agent', name: OWNER },
              initialContent: {
                kind: 'component',
                rendererId: 'operator_projection',
                props: { ...acceptanceProjection },
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
              detail: `${result.message}; projection=${acceptanceProjection.projection_id}; session=${session?.surface_session_id ?? 'none'}`,
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
