import { describe, expect, it } from 'vitest'
import type { MonitorContentDescriptor } from '../../../lib/monitorSurfaceContract'
import {
  resolveMonitorRenderer,
  type MonitorRendererTarget,
} from './monitorRendererRegistry'

function resolve(target: MonitorRendererTarget, descriptor: MonitorContentDescriptor) {
  return resolveMonitorRenderer({ target, descriptor })
}

describe('monitor renderer registry', () => {
  it('fails closed for malformed and unsafe descriptors', () => {
    const unsafeDescriptors: MonitorContentDescriptor[] = [
      { kind: 'web', url: 'file:///etc/passwd', display: 'inline', sandboxProfile: 'default' },
      { kind: 'youtube', videoId: '../not-a-video-id' },
      { kind: 'document', documentKind: 'markdown', source: { kind: 'local', path: '/etc/passwd' } },
      { kind: 'document', documentKind: 'markdown', source: { kind: 'local', path: '~/secret.md' } },
      { kind: 'document', documentKind: 'markdown', source: { kind: 'local', path: 'docs/../secret.md' } },
      { kind: 'document', documentKind: 'markdown', source: { kind: 'local', path: 'docs\\secret.md' } },
      { kind: 'web', url: 'https://example.invalid/dashboard', display: 'inline', sandboxProfile: 'unknown_profile' },
      { kind: 'terminal', sessionId: '   ' },
      {
        kind: 'remote_session',
        sessionId: 'remote-main',
        streamUrl: 'ftp://example.invalid/stream',
        transport: 'mjpeg',
      },
      {
        kind: 'remote_session',
        sessionId: 'remote-main',
        streamUrl: 'https://example.invalid/stream',
        transport: 'webrtc',
      },
    ]

    for (const descriptor of unsafeDescriptors) {
      const plan = resolve('aperture', descriptor)
      if (plan.ok) {
        throw new Error(`expected descriptor '${descriptor.kind}' to fail closed`)
      }
      expect(plan.reason).toMatch(/unsafe|invalid|unsupported|empty/i)
    }
  })

  it('keeps aperture web and youtube honest unless a capture stream is declared', () => {
    const web = resolve('aperture', {
      kind: 'web',
      url: 'https://example.invalid/dashboard',
      display: 'inline',
      sandboxProfile: 'default',
    })
    expect(web).toMatchObject({
      ok: false,
      kind: 'unsupported',
      reason: expect.stringContaining('capture stream required'),
    })

    const youtube = resolve('aperture', { kind: 'youtube', videoId: 'dQw4w9WgXcQ' })
    expect(youtube).toMatchObject({
      ok: false,
      kind: 'unsupported',
      reason: expect.stringContaining('capture stream required'),
    })

    const remoteCapture = resolve('aperture', {
      kind: 'remote_session',
      sessionId: 'capture-main',
      streamUrl: 'https://streams.example.invalid/capture.m3u8',
      transport: 'hls',
    })
    expect(remoteCapture).toMatchObject({
      ok: true,
      adapter: 'video_texture',
      muted: true,
      source: { url: 'https://streams.example.invalid/capture.m3u8', transport: 'hls' },
    })
  })

  it('resolves workstation web and youtube to iframe/embed plans with safe defaults', () => {
    const web = resolve('workstation', {
      kind: 'web',
      url: 'https://example.invalid/dashboard',
      display: 'inline',
      sandboxProfile: 'default',
    })
    expect(web).toMatchObject({
      ok: true,
      adapter: 'iframe',
      muted: true,
      source: { url: 'https://example.invalid/dashboard' },
    })

    const youtube = resolve('workstation', {
      kind: 'youtube',
      videoId: 'dQw4w9WgXcQ',
      startSeconds: 42,
      autoplay: true,
    })
    expect(youtube).toMatchObject({
      ok: true,
      adapter: 'youtube_embed',
      muted: true,
      source: { videoId: 'dQw4w9WgXcQ', startSeconds: 42, autoplay: true },
    })
  })

  it('resolves document and terminal descriptors to bounded existing adapters', () => {
    const doc = resolve('aperture', {
      kind: 'document',
      documentKind: 'pdf',
      source: { kind: 'local', path: 'docs/operator/runbook.pdf' },
      page: 2,
    })
    expect(doc).toMatchObject({
      ok: true,
      adapter: 'document_texture',
      source: { documentKind: 'pdf', page: 2 },
    })

    const terminal = resolve('workstation', {
      kind: 'terminal',
      sessionId: 'session-main',
      readOnly: true,
      theme: 'dark',
    })
    expect(terminal).toMatchObject({
      ok: true,
      adapter: 'terminal_session',
      source: { sessionId: 'session-main', readOnly: true, theme: 'dark' },
    })
  })

  it('registers image and video textures and fails closed for unregistered components and fallbacks', () => {
    expect(resolve('aperture', {
      kind: 'image',
      source: { kind: 'remote', url: 'https://media.example.invalid/frame.png' },
      fit: 'cover',
    })).toMatchObject({ ok: true, adapter: 'image_texture', muted: true })
    expect(resolve('aperture', {
      kind: 'video',
      source: { kind: 'remote', url: 'https://media.example.invalid/clip.mp4' },
      fit: 'contain',
      autoplay: true,
    })).toMatchObject({ ok: true, adapter: 'video_texture', muted: true })
    expect(resolve('aperture', {
      kind: 'component',
      rendererId: 'operator_projection',
      props: { projection_id: 'projection-p9-fixture' },
    })).toMatchObject({
      ok: true,
      adapter: 'component_canvas',
      source: {
        rendererId: 'operator_projection',
        props: { projection_id: 'projection-p9-fixture' },
        authority: 'read_only',
      },
    })
    expect(resolve('aperture', {
      kind: 'component',
      rendererId: 'unknown',
      props: {},
    })).toMatchObject({ ok: false, kind: 'unsupported' })
    expect(resolve('aperture', {
      kind: 'fallback',
      reason: 'capture offline',
      retryable: true,
    })).toMatchObject({ ok: false, reason: 'capture offline' })
  })
})
