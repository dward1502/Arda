import { describe, expect, it } from 'vitest'
import {
  claimFromMonitorEvent,
  formatMonitorSurfaceStream,
  normalizeMonitorLeaseExpiry,
  resolveMonitorContractSlotId,
  resolveMonitorSurfaceOpenRequest,
} from './monitorSurfaceRuntime'

describe('monitor surface runtime bridge', () => {
  it('renders real payload content instead of a synthetic timer tick', () => {
    expect(formatMonitorSurfaceStream({ content: 'provider healthy', mime: 'text/plain' }, false)).toBe('provider healthy')
    expect(formatMonitorSurfaceStream(null, false)).toBe('[NO DATA — awaiting agent payload]')
    expect(formatMonitorSurfaceStream({ content: 'still visible', mime: 'text/plain' }, true)).toBe('still visible')
  })

  it('normalizes unix lease values emitted by the Rust API', () => {
    expect(normalizeMonitorLeaseExpiry('unix:1785413400')).toBe('2026-07-30T12:10:00.000Z')
    expect(normalizeMonitorLeaseExpiry('2026-07-30T12:10:00.000Z')).toBe('2026-07-30T12:10:00.000Z')
  })

  it('maps an authorized native claim event into the persisted boardroom contract', () => {
    const fallback = { mode: 'agent_activity' as const, refresh_ms: 1000, widgets: [] }
    expect(claimFromMonitorEvent({
      slotId: 'monitor_1',
      owner: 'hermes-agent-001',
      activityKind: 'agent_activity',
      payloadBinding: 'hermes.live_stream',
      focusMode: 'remote_preview',
      leaseExpiresAtUtc: 'unix:1785413400',
      active: true,
    }, fallback)).toEqual({
      owner: 'hermes-agent-001',
      activity_kind: 'agent_activity',
      payload_binding: 'hermes.live_stream',
      fallback_preview: fallback,
      lease_expires_at_utc: '2026-07-30T12:10:00.000Z',
    })
  })

  it('routes monitor focus to a scoped native surface without changing desk behavior', () => {
    expect(resolveMonitorSurfaceOpenRequest('monitor_1', 'hermes.live_stream', 'remote_preview')).toEqual({
      slotId: 'monitor_1',
      sourceZoneId: 'hermes_runtime',
      focusMode: 'remote_preview',
      title: 'ARDA Monitor — monitor_1',
    })
    expect(resolveMonitorSurfaceOpenRequest('view_desk_l', 'systems_health', 'native_window')).toBeNull()
  })

  it('uses the assignment slot contract instead of the spatial scene-zone id', () => {
    expect(resolveMonitorContractSlotId('boardroom.monitor.left', 'monitor_1')).toBe('monitor_1')
    expect(resolveMonitorContractSlotId('monitor_center', undefined)).toBe('monitor_center')
  })
})
