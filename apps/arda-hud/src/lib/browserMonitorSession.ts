import type { AgentSurfaceOwner, UpperMonitorSlotId } from './monitorSurfaceContract'

export interface BrowserCaptureDescriptor {
  sessionId: string
  owner: string
  revision: number
  url: string
  streamUrl: string
  transport: 'mjpeg'
  muted: boolean
  processId: number
  frameRevision: number
}

export interface BrowserCaptureFrame {
  revision: number
  jpegBase64: string
}

export interface BrowserMonitorSessionRequest {
  slotId: UpperMonitorSlotId
  owner: AgentSurfaceOwner
  url: string
  ttlMs: number
  captureSessionId: string
}

interface BrowserCaptureIdentity {
  sessionId: string
  owner: string
}

interface BrowserCaptureControlIdentity extends BrowserCaptureIdentity {
  expectedRevision: number
}

export interface BrowserPointerPosition {
  x: number
  y: number
}

interface SurfaceClaimResult {
  ok: boolean
  session: { surface_session_id: string } | null
  message?: string
}

interface BrowserMonitorSessionDependencies {
  startCapture: (request: BrowserCaptureIdentity & { url: string }) => Promise<BrowserCaptureDescriptor>
  stopCapture: (request: BrowserCaptureIdentity) => Promise<unknown>
  claimSurface: (request: {
    slotId: UpperMonitorSlotId
    owner: AgentSurfaceOwner
    initialContent: {
      kind: 'remote_session'
      sessionId: string
      streamUrl: string
      transport: 'mjpeg'
    }
    ttlMs: number
  }) => Promise<SurfaceClaimResult>
}

export interface BrowserMonitorSessionResult {
  capture: BrowserCaptureDescriptor
  claim: SurfaceClaimResult
}

function serializeOwner(owner: AgentSurfaceOwner): string {
  if (owner.kind === 'agent') return `agent:${owner.name}`
  if (owner.kind === 'operator') return `operator:${owner.id}`
  return `system:${owner.name}`
}

export async function startBrowserMonitorSession(
  request: BrowserMonitorSessionRequest,
  dependencies: BrowserMonitorSessionDependencies,
): Promise<BrowserMonitorSessionResult> {
  const identity = {
    sessionId: request.captureSessionId,
    owner: serializeOwner(request.owner),
  }
  const capture = await dependencies.startCapture({ ...identity, url: request.url })
  try {
    if (!capture.muted || capture.frameRevision < 2 || capture.transport !== 'mjpeg') {
      throw new Error('browser capture did not prove two changing muted MJPEG frames')
    }
    const claim = await dependencies.claimSurface({
      slotId: request.slotId,
      owner: request.owner,
      initialContent: {
        kind: 'remote_session',
        sessionId: capture.sessionId,
        streamUrl: capture.streamUrl,
        transport: capture.transport,
      },
      ttlMs: request.ttlMs,
    })
    if (!claim.ok || !claim.session) {
      throw new Error(claim.message || 'browser monitor surface claim failed')
    }
    return { capture, claim }
  } catch (error) {
    await dependencies.stopCapture(identity)
    throw error
  }
}

export async function agentStartBrowserMonitorSession(
  request: BrowserMonitorSessionRequest,
): Promise<BrowserMonitorSessionResult> {
  const [{ invoke }, { agentClaimMonitorSurface }] = await Promise.all([
    import('@tauri-apps/api/core'),
    import('./boardroomSlotSettings'),
  ])
  return startBrowserMonitorSession(request, {
    startCapture: (captureRequest) => invoke<BrowserCaptureDescriptor>('start_browser_capture', {
      request: captureRequest,
    }),
    stopCapture: (captureRequest) => invoke('stop_browser_capture', {
      request: captureRequest,
    }),
    claimSurface: agentClaimMonitorSurface,
  })
}

export async function agentStopBrowserMonitorSession(
  identity: BrowserCaptureIdentity,
): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('stop_browser_capture', { request: identity })
}

export async function agentGetBrowserMonitorSession(
  sessionId: string,
): Promise<BrowserCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<BrowserCaptureDescriptor>('get_browser_capture_status', { sessionId })
}

export async function agentGetBrowserMonitorFrame(
  sessionId: string,
  afterRevision: number,
): Promise<BrowserCaptureFrame | null> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<BrowserCaptureFrame | null>('get_browser_capture_frame', {
    sessionId,
    afterRevision,
  })
}

export async function navigateBrowserMonitorSession(
  capture: BrowserCaptureDescriptor,
  url: string,
  navigateCapture: (
    request: BrowserCaptureControlIdentity & { url: string },
  ) => Promise<BrowserCaptureDescriptor>,
): Promise<BrowserCaptureDescriptor> {
  return navigateCapture({
    sessionId: capture.sessionId,
    owner: capture.owner,
    expectedRevision: capture.revision,
    url,
  })
}

export async function clickBrowserMonitorSession(
  capture: BrowserCaptureDescriptor,
  position: BrowserPointerPosition,
  clickCapture: (
    request: BrowserCaptureControlIdentity & BrowserPointerPosition,
  ) => Promise<BrowserCaptureDescriptor>,
): Promise<BrowserCaptureDescriptor> {
  return clickCapture({
    sessionId: capture.sessionId,
    owner: capture.owner,
    expectedRevision: capture.revision,
    ...position,
  })
}

export async function agentNavigateBrowserMonitorSession(
  capture: BrowserCaptureDescriptor,
  url: string,
): Promise<BrowserCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return navigateBrowserMonitorSession(capture, url, (request) =>
    invoke<BrowserCaptureDescriptor>('navigate_browser_capture', { request }))
}

export async function agentClickBrowserMonitorSession(
  capture: BrowserCaptureDescriptor,
  position: BrowserPointerPosition,
): Promise<BrowserCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return clickBrowserMonitorSession(capture, position, (request) =>
    invoke<BrowserCaptureDescriptor>('click_browser_capture', { request }))
}

export async function agentScrollBrowserMonitorSession(
  capture: BrowserCaptureDescriptor,
  input: BrowserPointerPosition & { deltaX: number; deltaY: number },
): Promise<BrowserCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<BrowserCaptureDescriptor>('scroll_browser_capture', {
    request: {
      sessionId: capture.sessionId,
      owner: capture.owner,
      expectedRevision: capture.revision,
      ...input,
    },
  })
}

export async function agentTypeBrowserMonitorSession(
  capture: BrowserCaptureDescriptor,
  text: string,
): Promise<BrowserCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<BrowserCaptureDescriptor>('type_browser_capture', {
    request: {
      sessionId: capture.sessionId,
      owner: capture.owner,
      expectedRevision: capture.revision,
      text,
    },
  })
}

export async function agentKeyBrowserMonitorSession(
  capture: BrowserCaptureDescriptor,
  key: string,
): Promise<BrowserCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<BrowserCaptureDescriptor>('key_browser_capture', {
    request: {
      sessionId: capture.sessionId,
      owner: capture.owner,
      expectedRevision: capture.revision,
      key,
    },
  })
}
