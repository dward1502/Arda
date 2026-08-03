import { describe, expect, it } from 'vitest'
import { resolveMonitorFocus } from './BoardroomViewport'
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

function makeSource(claim: BoardroomAgentClaim | null, active: boolean): BoardroomMonitorSlotSource {
  return {
    sourceZoneId: claim?.payload_binding ?? 'service_warp_dev',
    assignment: {
      slot_id: 'monitor_left_1',
      component_id: 'test',
      source_zone_id: 'service_warp_dev',
      title: 'Test',
      module_ids: [],
      presentation_modes: ['in_scene'],
      surface_layout: {
        enabled: true, adapter_type: 'component_grid',
        preview: { mode: 'component_grid', refresh_ms: 3000, widgets: [] },
        focus: { mode: 'in_scene_workstation', target: 'service_warp_dev', refresh_ms: 1000 },
        embed: { url: null, allow_inline: false },
      },
      visualization: { preset_id: 'standby', config: { density: 'medium', timespan_minutes: 15, alert_threshold: null } },
      lease_expires_at_utc: '2026-07-30T12:00:00.000Z',
    } as unknown as BoardroomMonitorSlotSource['assignment'],
    claim,
    active,
  }
}

describe('resolveMonitorFocus', () => {
  it('returns null for non-monitor (desk) slots', () => {
    expect(resolveMonitorFocus('view_desk_l', {}, {}, {}, {}, NOW)).toBeNull()
  })

  it('resolves the active claim source when a live binding exists', () => {
    const claim = makeClaim()
    const sources: Record<string, BoardroomMonitorSlotSource | null> = {
      monitor_left_1: makeSource(claim, true),
    }
    const claims = { monitor_left_1: claim }
    const assignments = { monitor_left_1: 'service_warp_dev' }
    const result = resolveMonitorFocus('monitor_left_1', assignments, sources, {}, claims, NOW)
    expect(result).not.toBeNull()
    expect(result!.hasActiveClaim).toBe(true)
    expect(result!.sourceZoneId).toBe('hermes.live_stream')
  })

  it('falls back to persisted assignment when no live claim is active', () => {
    const assignments = { monitor_left_1: 'service_warp_dev' }
    const result = resolveMonitorFocus('monitor_left_1', assignments, { monitor_left_1: null }, {}, {}, NOW)
    expect(result).not.toBeNull()
    expect(result!.hasActiveClaim).toBe(false)
    expect(result!.sourceZoneId).toBe('service_warp_dev')
  })

  it('falls back to default focus mode when no surface layout exists', () => {
    const assignments = { monitor_left_1: 'routing_and_comms' }
    const result = resolveMonitorFocus('monitor_left_1', assignments, {}, {}, {}, NOW)
    expect(result).not.toBeNull()
    expect(result!.hasActiveClaim).toBe(false)
    expect(result!.focusMode).toBe('in_scene_workstation')
  })

  it('ignores inactive claims and falls back to persisted assignment', () => {
    const expiredClaim = makeClaim({ lease_expires_at_utc: '2026-01-01T00:00:00.000Z' })
    const sources: Record<string, BoardroomMonitorSlotSource | null> = {
      monitor_left_1: makeSource(expiredClaim, false),
    }
    const result = resolveMonitorFocus('monitor_left_1', { monitor_left_1: 'service_warp_dev' }, sources, {}, {}, NOW)
    expect(result!.hasActiveClaim).toBe(false)
    expect(result!.sourceZoneId).toBe('service_warp_dev')
  })

  it('uses layout focus mode when available', () => {
    const assignments = { monitor_left_1: 'service_warp_dev' }
    const layouts = {
      monitor_left_1: {
        enabled: true,
        adapter_type: 'service_embed' as const,
        preview: { mode: 'stream_feed' as const, refresh_ms: 2500, widgets: [] },
        focus: { mode: 'remote_preview' as const, target: 'service_warp_dev', refresh_ms: 1000 },
        embed: { url: 'http://example.com', allow_inline: false },
      },
    }
    const result = resolveMonitorFocus('monitor_left_1', assignments, {}, layouts, {}, NOW)
    expect(result!.focusMode).toBe('remote_preview')
    expect(result!.sourceZoneId).toBe('service_warp_dev')
  })
})
