import type { MonitorContentDescriptor, MonitorMediaSource } from '../../../lib/monitorSurfaceContract'

export type MonitorRendererTarget = 'aperture' | 'workstation'

export type MonitorRendererAdapter =
  | 'iframe'
  | 'youtube_embed'
  | 'capture_stream_required'
  | 'document_texture'
  | 'terminal_session'
  | 'image_texture'
  | 'video_texture'
  | 'component_canvas'
  | 'unsupported'

export type MonitorRemoteTransport = 'hls' | 'mjpeg'

export interface MonitorRendererResolveRequest {
  target: MonitorRendererTarget
  descriptor: MonitorContentDescriptor
}

export interface MonitorRendererFailurePlan {
  ok: false
  kind: 'unsupported' | 'invalid'
  adapter: 'unsupported'
  reason: string
}

export interface MonitorRendererSuccessPlan {
  ok: true
  kind: 'renderer'
  adapter: Exclude<MonitorRendererAdapter, 'capture_stream_required' | 'unsupported'>
  muted: boolean
  source: Record<string, unknown>
}

export type MonitorRendererPlan = MonitorRendererSuccessPlan | MonitorRendererFailurePlan

type DescriptorKind = MonitorContentDescriptor['kind']
type DescriptorForKind<K extends DescriptorKind> = Extract<MonitorContentDescriptor, { kind: K }>

interface RendererDefinition {
  kind: DescriptorKind
  resolve: (target: MonitorRendererTarget, descriptor: MonitorContentDescriptor) => MonitorRendererPlan
}

function isDescriptorKind<K extends DescriptorKind>(
  descriptor: MonitorContentDescriptor,
  kind: K,
): descriptor is DescriptorForKind<K> {
  return descriptor.kind === kind
}

function defineRenderer<K extends DescriptorKind>(
  kind: K,
  resolve: (target: MonitorRendererTarget, descriptor: DescriptorForKind<K>) => MonitorRendererPlan,
): RendererDefinition {
  return {
    kind,
    resolve(target, descriptor) {
      if (!isDescriptorKind(descriptor, kind)) {
        return invalid(`renderer '${kind}' received descriptor kind '${descriptor.kind}'`)
      }
      return resolve(target, descriptor)
    },
  }
}

function invalid(reason: string): MonitorRendererFailurePlan {
  return { ok: false, kind: 'invalid', adapter: 'unsupported', reason }
}

function unsupported(reason: string): MonitorRendererFailurePlan {
  return { ok: false, kind: 'unsupported', adapter: 'unsupported', reason }
}

function renderer(
  adapter: MonitorRendererSuccessPlan['adapter'],
  source: Record<string, unknown>,
): MonitorRendererSuccessPlan {
  return { ok: true, kind: 'renderer', adapter, muted: true, source }
}

function isHttpUrl(value: string): boolean {
  try {
    const url = new URL(value)
    return url.protocol === 'http:' || url.protocol === 'https:'
  } catch {
    return false
  }
}

function isSafeYoutubeVideoId(value: string): boolean {
  return /^[A-Za-z0-9_-]{11}$/.test(value)
}

function isNonEmptyIdentifier(value: string): boolean {
  return value.trim().length > 0
}

const APPROVED_WEB_SANDBOX_PROFILES = ['default', 'trusted_embed', 'capture_stream'] as const

type ApprovedWebSandboxProfile = typeof APPROVED_WEB_SANDBOX_PROFILES[number]

function isApprovedWebSandboxProfile(value: string): value is ApprovedWebSandboxProfile {
  return APPROVED_WEB_SANDBOX_PROFILES.includes(value as ApprovedWebSandboxProfile)
}

function validateWebSandboxProfile(value: string): string | null {
  return isApprovedWebSandboxProfile(value) ? null : `unsupported web sandbox profile '${value}'`
}

function validateMediaSource(source: MonitorMediaSource): string | null {
  if (source.kind === 'remote') {
    return isHttpUrl(source.url) ? null : `unsafe remote document URL '${source.url}'`
  }

  const path = source.path.trim()
  if (!path) return 'empty local document path'
  const segments = path.split('/').filter(Boolean)
  if (
    path.startsWith('/') ||
    path.startsWith('~') ||
    path.includes('\\') ||
    segments.some((segment) => segment === '..')
  ) {
    return `unsafe local document path '${source.path}'`
  }
  return null
}

function isSupportedRemoteTransport(value: string): value is MonitorRemoteTransport {
  return value === 'hls' || value === 'mjpeg'
}

const rendererDefinitions = [
  defineRenderer('web', (target, descriptor) => {
    if (!isHttpUrl(descriptor.url)) return invalid(`unsafe web URL '${descriptor.url}'`)
    const sandboxError = validateWebSandboxProfile(descriptor.sandboxProfile)
    if (sandboxError) return invalid(sandboxError)
    if (target === 'aperture') {
      return unsupported('capture stream required for aperture web content; direct iframe pixels cannot be rendered into WebGL')
    }
    return renderer('iframe', {
      url: descriptor.url,
      title: descriptor.title,
      sandboxProfile: descriptor.sandboxProfile,
      display: descriptor.display,
    })
  }),
  defineRenderer('youtube', (target, descriptor) => {
    if (!isSafeYoutubeVideoId(descriptor.videoId)) return invalid(`invalid YouTube video id '${descriptor.videoId}'`)
    if (target === 'aperture') {
      return unsupported('capture stream required for aperture YouTube content; direct iframe pixels cannot be rendered into WebGL')
    }
    return renderer('youtube_embed', {
      videoId: descriptor.videoId,
      startSeconds: descriptor.startSeconds,
      autoplay: descriptor.autoplay ?? false,
      muted: true,
    })
  }),
  defineRenderer('video', (_target, descriptor) => {
    const sourceError = validateMediaSource(descriptor.source)
    if (sourceError) return invalid(sourceError)
    return renderer('video_texture', {
      source: descriptor.source,
      mime: descriptor.mime,
      fit: descriptor.fit,
      loop: descriptor.loop ?? false,
      autoplay: descriptor.autoplay ?? false,
      muted: true,
    })
  }),
  defineRenderer('image', (_target, descriptor) => {
    const sourceError = validateMediaSource(descriptor.source)
    if (sourceError) return invalid(sourceError)
    return renderer('image_texture', {
      source: descriptor.source,
      fit: descriptor.fit,
      alt: descriptor.alt,
    })
  }),
  defineRenderer('document', (_target, descriptor) => {
    const sourceError = validateMediaSource(descriptor.source)
    if (sourceError) return invalid(sourceError)
    return renderer('document_texture', {
      source: descriptor.source,
      documentKind: descriptor.documentKind,
      page: descriptor.page ?? 1,
    })
  }),
  defineRenderer('terminal', (_target, descriptor) => {
    if (!isNonEmptyIdentifier(descriptor.sessionId)) return invalid('empty terminal session id')
    return renderer('terminal_session', {
      sessionId: descriptor.sessionId,
      readOnly: descriptor.readOnly ?? true,
      theme: descriptor.theme,
      subscription: 'existing_named_pty_session',
    })
  }),
  defineRenderer('remote_session', (_target, descriptor) => {
    if (!isNonEmptyIdentifier(descriptor.sessionId)) return invalid('empty remote session id')
    if (!isSupportedRemoteTransport(descriptor.transport)) {
      return unsupported(`unsupported remote session transport '${String(descriptor.transport)}'`)
    }
    if (!isHttpUrl(descriptor.streamUrl)) return invalid(`unsafe remote session stream URL '${descriptor.streamUrl}'`)
    return renderer(descriptor.transport === 'hls' ? 'video_texture' : 'image_texture', {
      sessionId: descriptor.sessionId,
      url: descriptor.streamUrl,
      transport: descriptor.transport,
    })
  }),
  defineRenderer('component', (_target, descriptor) => {
    if (!isNonEmptyIdentifier(descriptor.rendererId)) return invalid('empty component renderer id')
    if (descriptor.rendererId === 'operator_projection') {
      return renderer('component_canvas', {
        rendererId: descriptor.rendererId,
        props: descriptor.props,
        authority: 'read_only',
      })
    }
    return unsupported(`trusted component renderer '${descriptor.rendererId}' is not registered`)
  }),
  defineRenderer('fallback', (_target, descriptor) => unsupported(descriptor.reason || 'fallback content requested')),
] satisfies readonly RendererDefinition[]

export const MONITOR_RENDERER_REGISTRY = rendererDefinitions

export function resolveMonitorRenderer(request: MonitorRendererResolveRequest): MonitorRendererPlan {
  const definition = MONITOR_RENDERER_REGISTRY.find(
    (entry) => entry.kind === request.descriptor.kind,
  )
  if (!definition) {
    return unsupported(`unsupported monitor descriptor kind '${request.descriptor.kind}'`)
  }
  return definition.resolve(request.target, request.descriptor)
}
