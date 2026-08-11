import type { MonitorContentDescriptor } from '../../lib/monitorSurfaceContract'
import { resolveMonitorRenderer } from './renderers/monitorRendererRegistry'

export type MonitorApertureDescriptorState =
  | { mode: 'render' }
  | { mode: 'message'; title: string; detail: string; color: string }

export function resolveMonitorApertureDescriptorState(
  descriptor: MonitorContentDescriptor,
): MonitorApertureDescriptorState {
  const plan = resolveMonitorRenderer({ target: 'aperture', descriptor })
  if (plan.ok === false) {
    return {
      mode: 'message',
      title: plan.reason.includes('capture stream required')
        ? 'CAPTURE STREAM REQUIRED'
        : 'CONTENT UNAVAILABLE',
      detail: plan.reason,
      color: plan.kind === 'invalid' ? '#ff789c' : '#ffd37a',
    }
  }

  if (descriptor.kind === 'document') {
    const detail = descriptor.source.kind === 'local' ? descriptor.source.path : descriptor.source.url
    if (descriptor.documentKind === 'pdf') {
      return { mode: 'message', title: 'PDF PREVIEW REQUIRED', detail, color: '#ffd37a' }
    }
    if (descriptor.source.kind === 'remote') {
      return { mode: 'message', title: 'REMOTE DOCUMENT UNAVAILABLE', detail, color: '#ffd37a' }
    }
  }

  if (descriptor.kind === 'terminal') {
    return {
      mode: 'message',
      title: 'TERMINAL SESSION UNAVAILABLE',
      detail: descriptor.sessionId,
      color: '#ffd37a',
    }
  }

  return { mode: 'render' }
}
