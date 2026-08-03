import { describe, expect, it, beforeEach, afterEach } from 'vitest'
import { resolveMonitorFocus, shouldRenderActiveMonitorClaim } from './BoardroomViewport'
import type { BoardroomAgentClaim, BoardroomMonitorSlotSource } from '../../lib/boardroomSlotSettings'

const NOW = '2026-07-30T12:00:00.000Z'

function makeClaim(overrides: Partial<BoardroomAgentClaim> = {}): BoardroomAgentClaim {
  return {
    owner: 'hermes-agent-001',
    activity_kind: 'agent_activity',
    payload_binding: 'hermes.live_stream',
    fallback_preview: { mode: 'agent_activity', refresh_ms: 1000, widgets: [] },
    lease_expires_at_utc: '2026-12-31T23:59:59.000Z',
    ...overrides,
  }
}

function makeSource(): BoardroomMonitorSlotSource {
  return {
    sourceZoneId: 'hermes.live_stream',
    assignment: {
      slot_id: 'monitor_left_1',
      component_id: 'test',
      source_zone_id: 'hermes.live_stream',
      title: 'Hermes Live',
      module_ids: [],
      presentation_modes: ['in_scene'],
      surface_layout: {
        enabled: true, adapter_type: 'agent_activity',
        preview: { mode: 'agent_activity', refresh_ms: 1000, widgets: [] },
        focus: { mode: 'remote_preview', target: 'hermes.live_stream', refresh_ms: 1000 },
        embed: { url: null, allow_inline: false },
      },
      visualization: { preset_id: 'standby', config: { density: 'medium', timespan_minutes: 15, alert_threshold: null } },
      lease_expires_at_utc: '2026-07-30T12:00:00.000Z',
    } as unknown as BoardroomMonitorSlotSource['assignment'],
    claim: makeClaim(),
    active: true,
  }
}

describe('HermesDashboardMonitorSurface — reduced-motion policy', () => {
  const originalMatchMedia = window.matchMedia

  beforeEach(() => {
    window.matchMedia = originalMatchMedia
  })

  afterEach(() => {
    window.matchMedia = originalMatchMedia
  })

  it('resolveMonitorFocus returns remote_preview focus mode for active claim with hermes.live_stream binding', () => {
    const claim = makeClaim()
    const sources: Record<string, BoardroomMonitorSlotSource | null> = {
      monitor_left_1: makeSource(),
    }
    const claims = { monitor_left_1: claim }
    const assignments = { monitor_left_1: 'hermes.live_stream' }
    const layouts = {
      monitor_left_1: {
        enabled: true, adapter_type: 'agent_activity',
        preview: { mode: 'agent_activity', refresh_ms: 1000, widgets: [] },
        focus: { mode: 'remote_preview', target: 'hermes.live_stream', refresh_ms: 1000 },
        embed: { url: null, allow_inline: false },
      },
    }
    const result = resolveMonitorFocus('monitor_left_1', assignments, sources, layouts, claims, NOW)
    expect(result).not.toBeNull()
    expect(result!.hasActiveClaim).toBe(true)
    expect(result!.focusMode).toBe('remote_preview')
    expect(result!.sourceZoneId).toBe('hermes.live_stream')
  })

  it('renders an active claim preview independently of its configured focus behavior', () => {
    expect(shouldRenderActiveMonitorClaim({ hasActiveClaim: true, focusMode: 'native_window', sourceZoneId: 'hermes.live_stream' })).toBe(true)
    expect(shouldRenderActiveMonitorClaim({ hasActiveClaim: false, focusMode: 'native_window', sourceZoneId: 'service_warp_dev' })).toBe(false)
  })

  it('resolveMonitorFocus ignores desk slots for agent surface', () => {
    const result = resolveMonitorFocus('view_desk_l', {}, {}, {}, {}, NOW)
    expect(result).toBeNull()
  })

  it('resolveMonitorFocus falls back to persisted when claim lease is expired', () => {
    const expiredClaim = makeClaim({ lease_expires_at_utc: '2026-01-01T00:00:00.000Z' })
    const sources: Record<string, BoardroomMonitorSlotSource | null> = {
      monitor_left_1: { ...makeSource(), claim: expiredClaim, active: false },
    }
    const result = resolveMonitorFocus(
      'monitor_left_1',
      { monitor_left_1: 'service_warp_dev' },
      sources,
      {},
      { monitor_left_1: expiredClaim },
      NOW,
    )
    expect(result!.hasActiveClaim).toBe(false)
    expect(result!.sourceZoneId).toBe('service_warp_dev')
  })

  it('resolveMonitorFocus returns remote_preview focus when layout specifies it with no active claim', () => {
    const result = resolveMonitorFocus(
      'monitor_left_1',
      { monitor_left_1: 'hermes.live_stream' },
      {},
      {
        monitor_left_1: {
          enabled: true, adapter_type: 'agent_activity',
          preview: { mode: 'agent_activity', refresh_ms: 1000, widgets: [] },
          focus: { mode: 'remote_preview', target: 'hermes.live_stream', refresh_ms: 1000 },
          embed: { url: null, allow_inline: false },
        },
      },
      {},
      NOW,
    )
    expect(result).not.toBeNull()
    expect(result!.focusMode).toBe('remote_preview')
    expect(result!.sourceZoneId).toBe('hermes.live_stream')
  })
})
