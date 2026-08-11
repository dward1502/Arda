import type { MonitorMediaSource, MonitorPlaybackState } from '../../lib/monitorSurfaceContract'

export type MonitorMediaUrlResult =
  | { ok: true; url: string }
  | { ok: false; reason: string }

export interface VideoPlaybackPlan {
  playing: boolean
  seekTo: number | null
  volume: number
}

export interface GeneratedFrameProps {
  frameId: string
  source: MonitorMediaSource
  fit: 'contain' | 'cover'
}

interface MjpegFrameSource {
  naturalWidth: number
  naturalHeight: number
}

export function pumpMjpegFrames(
  source: MjpegFrameSource,
  drawFrame: () => void,
  scheduleFrame: (next: () => void) => void,
  isActive: () => boolean,
): void {
  if (!isActive()) return
  if (source.naturalWidth > 0 && source.naturalHeight > 0) drawFrame()
  scheduleFrame(() => pumpMjpegFrames(source, drawFrame, scheduleFrame, isActive))
}

export function parseGeneratedFrameProps(value: unknown): GeneratedFrameProps | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const props = value as Record<string, unknown>
  if (typeof props.frameId !== 'string' || props.frameId.trim().length === 0) return null
  if (!props.source || typeof props.source !== 'object' || Array.isArray(props.source)) return null
  const source = props.source as Record<string, unknown>
  let mediaSource: MonitorMediaSource
  if (source.kind === 'local' && typeof source.path === 'string' && isSafeLocalMediaPath(source.path)) {
    mediaSource = { kind: 'local', path: source.path }
  } else if (source.kind === 'remote' && typeof source.url === 'string' && isApprovedRemoteMediaUrl(source.url)) {
    mediaSource = { kind: 'remote', url: source.url }
  } else {
    return null
  }
  if (props.fit !== undefined && props.fit !== 'contain' && props.fit !== 'cover') return null
  return {
    frameId: props.frameId,
    source: mediaSource,
    fit: props.fit === 'cover' ? 'cover' : 'contain',
  }
}

function isSafeLocalMediaPath(path: string): boolean {
  const trimmed = path.trim()
  if (!trimmed || trimmed.startsWith('/') || trimmed.startsWith('~') || trimmed.includes('\\')) return false
  return !trimmed.split('/').some((segment) => segment === '..')
}

function isApprovedRemoteMediaUrl(value: string): boolean {
  try {
    const url = new URL(value)
    return url.protocol === 'http:' || url.protocol === 'https:'
  } catch {
    return false
  }
}

export function resolveMonitorMediaUrl(
  source: MonitorMediaSource,
  rootPath: string | null | undefined,
  convertLocalPath: (absolutePath: string) => string,
): MonitorMediaUrlResult {
  if (source.kind === 'remote') {
    return isApprovedRemoteMediaUrl(source.url)
      ? { ok: true, url: source.url }
      : { ok: false, reason: 'remote monitor media requires an HTTP(S) URL' }
  }
  if (!isSafeLocalMediaPath(source.path)) {
    return { ok: false, reason: 'local monitor media path is outside the workspace boundary' }
  }
  if (!rootPath?.trim()) {
    return { ok: false, reason: 'workspace root is required for local monitor media' }
  }
  const absolutePath = `${rootPath.replace(/\/$/, '')}/${source.path}`
  return { ok: true, url: convertLocalPath(absolutePath) }
}

export function resolveVideoPlaybackPlan(
  playback: MonitorPlaybackState | undefined,
  autoplay: boolean,
): VideoPlaybackPlan {
  const currentTime = playback?.currentTime
  const seekTo = currentTime == null
    ? null
    : Number.isFinite(currentTime) ? Math.max(0, currentTime) : 0
  const volume = playback?.volume
  return {
    playing: playback?.playing ?? autoplay,
    seekTo,
    volume: volume == null || !Number.isFinite(volume)
      ? 0
      : Math.min(1, Math.max(0, volume)),
  }
}
