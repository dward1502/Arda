import { describe, expect, it, vi } from 'vitest'
import {
  pumpMjpegFrames,
  parseGeneratedFrameProps,
  resolveMjpegRenderPath,
  resolveMonitorMediaUrl,
  resolveVideoPlaybackPlan,
} from './monitorMediaRuntime'

describe('monitor media runtime', () => {
  it('routes owned loopback browser sessions through native frame delivery', () => {
    expect(resolveMjpegRenderPath(
      'http://127.0.0.1:46577/session/browser-monitor-1.mjpeg',
      'browser-monitor-1',
    )).toEqual({ kind: 'native-browser-frames', sessionId: 'browser-monitor-1' })

    expect(resolveMjpegRenderPath(
      'https://camera.example.invalid/live.mjpeg',
      'camera-1',
    )).toEqual({ kind: 'mjpeg-image' })
  })

  it('keeps drawing changing MJPEG pixels into the CanvasTexture aperture', () => {
    const source = { naturalWidth: 0, naturalHeight: 0 }
    const drawFrame = vi.fn()
    const scheduled: Array<() => void> = []
    let active = true

    pumpMjpegFrames(
      source,
      drawFrame,
      (next) => scheduled.push(next),
      () => active,
    )
    expect(drawFrame).not.toHaveBeenCalled()
    expect(scheduled).toHaveLength(1)

    source.naturalWidth = 1280
    source.naturalHeight = 720
    scheduled.shift()?.()
    expect(drawFrame).toHaveBeenCalledTimes(1)
    expect(scheduled).toHaveLength(1)

    scheduled.shift()?.()
    expect(drawFrame).toHaveBeenCalledTimes(2)
    active = false
    scheduled.shift()?.()
    expect(scheduled).toHaveLength(0)
  })

  it('keeps approved remote media URLs unchanged', () => {
    const convertLocalPath = vi.fn((path: string) => `asset://${path}`)

    expect(resolveMonitorMediaUrl(
      { kind: 'remote', url: 'https://media.example.invalid/frame.png' },
      '/arda',
      convertLocalPath,
    )).toEqual({ ok: true, url: 'https://media.example.invalid/frame.png' })
    expect(convertLocalPath).not.toHaveBeenCalled()
  })

  it('converts workspace-relative local media into a Tauri asset URL', () => {
    const convertLocalPath = vi.fn((path: string) => `asset://localhost${path}`)

    expect(resolveMonitorMediaUrl(
      { kind: 'local', path: 'docs/media/demo clip.mp4' },
      '/var/home/mythos/Eregion/Arda/',
      convertLocalPath,
    )).toEqual({
      ok: true,
      url: 'asset://localhost/var/home/mythos/Eregion/Arda/docs/media/demo clip.mp4',
    })
    expect(convertLocalPath).toHaveBeenCalledWith(
      '/var/home/mythos/Eregion/Arda/docs/media/demo clip.mp4',
    )
  })

  it('fails closed when a local source has no workspace root', () => {
    expect(resolveMonitorMediaUrl(
      { kind: 'local', path: 'docs/media/demo.mp4' },
      null,
      vi.fn(),
    )).toEqual({
      ok: false,
      reason: 'workspace root is required for local monitor media',
    })
  })

  it('rejects unsafe remote schemes and local traversal at the runtime boundary', () => {
    expect(resolveMonitorMediaUrl(
      { kind: 'remote', url: 'javascript:alert(1)' },
      '/arda',
      vi.fn(),
    )).toEqual({ ok: false, reason: 'remote monitor media requires an HTTP(S) URL' })
    expect(resolveMonitorMediaUrl(
      { kind: 'local', path: '../secret.mp4' },
      '/arda',
      vi.fn(),
    )).toEqual({ ok: false, reason: 'local monitor media path is outside the workspace boundary' })
  })

  it('derives synchronized playback from the authoritative session state', () => {
    expect(resolveVideoPlaybackPlan(
      { playing: false, currentTime: 42.5, duration: 120, volume: 2 },
      true,
    )).toEqual({ playing: false, seekTo: 42.5, volume: 1 })

    expect(resolveVideoPlaybackPlan(undefined, true)).toEqual({
      playing: true,
      seekTo: null,
      volume: 0,
    })
  })

  it('sanitizes invalid playback offsets and volume', () => {
    expect(resolveVideoPlaybackPlan(
      { playing: true, currentTime: -5, volume: Number.NaN },
      false,
    )).toEqual({ playing: true, seekTo: 0, volume: 0 })
  })

  it('accepts only bounded generated-frame references', () => {
    expect(parseGeneratedFrameProps({
      frameId: 'frame-42',
      source: { kind: 'local', path: 'state/generated/frame-42.webp' },
      fit: 'cover',
    })).toEqual({
      frameId: 'frame-42',
      source: { kind: 'local', path: 'state/generated/frame-42.webp' },
      fit: 'cover',
    })
    expect(parseGeneratedFrameProps({
      frameId: 'frame-unsafe',
      source: { kind: 'local', path: '../secrets.webp' },
    })).toBeNull()
    expect(parseGeneratedFrameProps({
      frameId: 'frame-script',
      source: { kind: 'remote', url: 'javascript:alert(1)' },
    })).toBeNull()
  })
})
