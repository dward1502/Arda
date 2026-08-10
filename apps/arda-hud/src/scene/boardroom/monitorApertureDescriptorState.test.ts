import { describe, expect, it } from 'vitest'
import { resolveMonitorApertureDescriptorState } from './monitorApertureDescriptorState'

describe('monitor aperture descriptor state', () => {
  it('reports capture requirements instead of pretending iframe pixels are available', () => {
    expect(resolveMonitorApertureDescriptorState({
      kind: 'web',
      url: 'https://example.invalid/app',
      display: 'inline',
      sandboxProfile: 'default',
    })).toMatchObject({ mode: 'message', title: 'CAPTURE STREAM REQUIRED' })
    expect(resolveMonitorApertureDescriptorState({
      kind: 'youtube',
      videoId: 'dQw4w9WgXcQ',
    })).toMatchObject({ mode: 'message', title: 'CAPTURE STREAM REQUIRED' })
  })

  it('names unavailable terminal sessions and bounded document limitations', () => {
    expect(resolveMonitorApertureDescriptorState({
      kind: 'terminal',
      sessionId: 'agent-main',
      readOnly: true,
    })).toEqual({
      mode: 'message',
      title: 'TERMINAL SESSION UNAVAILABLE',
      detail: 'agent-main',
      color: '#ffd37a',
    })
    expect(resolveMonitorApertureDescriptorState({
      kind: 'document',
      documentKind: 'pdf',
      source: { kind: 'local', path: 'docs/report.pdf' },
    })).toMatchObject({ mode: 'message', title: 'PDF PREVIEW REQUIRED', detail: 'docs/report.pdf' })
    expect(resolveMonitorApertureDescriptorState({
      kind: 'document',
      documentKind: 'markdown',
      source: { kind: 'local', path: 'docs/report.md' },
    })).toEqual({ mode: 'render' })
  })

  it('permits decodable capture transports and rejects unsupported WebRTC', () => {
    expect(resolveMonitorApertureDescriptorState({
      kind: 'remote_session',
      sessionId: 'capture-main',
      streamUrl: 'https://example.invalid/frame.mjpg',
      transport: 'mjpeg',
    })).toEqual({ mode: 'render' })
    expect(resolveMonitorApertureDescriptorState({
      kind: 'remote_session',
      sessionId: 'capture-main',
      streamUrl: 'https://example.invalid/live',
      transport: 'webrtc',
    })).toMatchObject({ mode: 'message', title: 'CONTENT UNAVAILABLE' })
  })
})
