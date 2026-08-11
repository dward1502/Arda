import { describe, expect, it, vi } from 'vitest'
import { startBrowserMonitorSession } from './browserMonitorSession'

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
})
