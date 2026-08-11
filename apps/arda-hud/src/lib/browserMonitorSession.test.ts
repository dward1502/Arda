import { describe, expect, it, vi } from 'vitest'
import {
  clickBrowserMonitorSession,
  navigateBrowserMonitorSession,
  startBrowserMonitorSession,
} from './browserMonitorSession'

describe('browser monitor session', () => {
  it('publishes a remote-session descriptor only after real changing muted frames exist', async () => {
    const startCapture = vi.fn().mockResolvedValue({
      sessionId: 'browser-monitor-1',
      owner: 'agent:browser-a',
      revision: 1,
      url: 'https://example.com',
      streamUrl: 'http://127.0.0.1:39111/session/browser-monitor-1.mjpeg',
      transport: 'mjpeg',
      muted: true,
      processId: 4242,
      frameRevision: 2,
    })
    const claimSurface = vi.fn().mockResolvedValue({ ok: true, session: { surface_session_id: 'surface-1' } })
    const stopCapture = vi.fn()

    const result = await startBrowserMonitorSession({
      slotId: 'monitor_1',
      owner: { kind: 'agent', name: 'browser-a' },
      url: 'https://example.com',
      ttlMs: 60_000,
      captureSessionId: 'browser-monitor-1',
    }, { startCapture, stopCapture, claimSurface })

    expect(claimSurface).toHaveBeenCalledWith(expect.objectContaining({
      slotId: 'monitor_1',
      initialContent: {
        kind: 'remote_session',
        sessionId: 'browser-monitor-1',
        streamUrl: 'http://127.0.0.1:39111/session/browser-monitor-1.mjpeg',
        transport: 'mjpeg',
      },
    }))
    expect(stopCapture).not.toHaveBeenCalled()
    expect(result.capture.frameRevision).toBe(2)
  })

  it('stops the owned browser when the authoritative surface claim fails', async () => {
    const capture = {
      sessionId: 'browser-monitor-2', owner: 'agent:browser-b', revision: 1,
      url: 'https://example.org', streamUrl: 'http://127.0.0.1:39112/session/browser-monitor-2.mjpeg',
      transport: 'mjpeg' as const, muted: true, processId: 4343, frameRevision: 3,
    }
    const stopCapture = vi.fn().mockResolvedValue(undefined)

    await expect(startBrowserMonitorSession({
      slotId: 'monitor_2',
      owner: { kind: 'agent', name: 'browser-b' },
      url: 'https://example.org',
      ttlMs: 60_000,
      captureSessionId: 'browser-monitor-2',
    }, {
      startCapture: vi.fn().mockResolvedValue(capture),
      stopCapture,
      claimSurface: vi.fn().mockResolvedValue({ ok: false, session: null, message: 'occupied' }),
    })).rejects.toThrow('occupied')

    expect(stopCapture).toHaveBeenCalledWith({ sessionId: 'browser-monitor-2', owner: 'agent:browser-b' })
  })

  it('navigates with the exact owner and current capture revision', async () => {
    const capture = {
      sessionId: 'browser-monitor-3', owner: 'agent:browser-c', revision: 4,
      url: 'https://example.com', streamUrl: 'http://127.0.0.1:39113/session/browser-monitor-3.mjpeg',
      transport: 'mjpeg' as const, muted: true, processId: 4444, frameRevision: 8,
    }
    const navigateCapture = vi.fn().mockResolvedValue({ ...capture, revision: 5, url: 'https://example.org' })

    const result = await navigateBrowserMonitorSession(capture, 'https://example.org', navigateCapture)

    expect(navigateCapture).toHaveBeenCalledWith({
      sessionId: capture.sessionId,
      owner: capture.owner,
      expectedRevision: 4,
      url: 'https://example.org',
    })
    expect(result.revision).toBe(5)
  })

  it('dispatches pointer input with the exact owner and current capture revision', async () => {
    const capture = {
      sessionId: 'browser-monitor-4', owner: 'agent:browser-d', revision: 9,
      url: 'https://example.com', streamUrl: 'http://127.0.0.1:39114/session/browser-monitor-4.mjpeg',
      transport: 'mjpeg' as const, muted: true, processId: 4545, frameRevision: 12,
    }
    const clickCapture = vi.fn().mockResolvedValue({ ...capture, revision: 10 })

    const result = await clickBrowserMonitorSession(capture, { x: 640, y: 360 }, clickCapture)

    expect(clickCapture).toHaveBeenCalledWith({
      sessionId: capture.sessionId,
      owner: capture.owner,
      expectedRevision: 9,
      x: 640,
      y: 360,
    })
    expect(result.revision).toBe(10)
  })
})
