import { describe, expect, it } from 'vitest'
import { DEFAULT_AGENT_PRESENCE_STATE } from '../systems/presenceState'
import type { PresenceLedgerStatus } from '../systems/presenceTypes'
import { deriveBoardroomPresenceStatusView } from './boardroomPresenceStatus'

function status(overrides: Partial<PresenceLedgerStatus> = {}): PresenceLedgerStatus {
  return {
    source: 'live_ledger',
    sourcePath: 'data/prometheus/arda_presence_events.jsonl',
    freshness: 'fresh',
    validEventCount: 1,
    ignoredLineCount: 0,
    malformedLineCount: 0,
    latestEventId: 'presence-1',
    latestTimestamp: '2026-07-30T20:01:00.000Z',
    ageSeconds: 300,
    summary: 'Live presence ledger fresh: event presence-1, 300s old',
    ...overrides,
  }
}

describe('deriveBoardroomPresenceStatusView', () => {
  it('discloses the canonical source path, observation time, and live freshness', () => {
    const view = deriveBoardroomPresenceStatusView(status(), {
      ...DEFAULT_AGENT_PRESENCE_STATE,
      primaryAgent: 'athena',
    })

    expect(view.label).toBe('Presence live')
    expect(view.detail).toBe('ATHENA · 20:01Z · fresh · 1 ledger row')
    expect(view.title).toContain('data/prometheus/arda_presence_events.jsonl · observed 2026-07-30T20:01:00.000Z')
    expect(view.className).toContain('--fresh')
  })

  it('keeps valid old records visible but marks them stale', () => {
    const view = deriveBoardroomPresenceStatusView(status({ freshness: 'stale' }), DEFAULT_AGENT_PRESENCE_STATE)

    expect(view.label).toBe('Presence stale')
    expect(view.detail).toContain('20:01Z · stale')
    expect(view.className).toContain('--stale')
  })

  it('shows malformed ledgers as missing fallback data rather than nominal presence', () => {
    const view = deriveBoardroomPresenceStatusView(status({
      source: 'fallback_default',
      freshness: 'unknown',
      validEventCount: 0,
      malformedLineCount: 2,
      latestEventId: undefined,
      latestTimestamp: undefined,
      ageSeconds: undefined,
      summary: 'Fallback default: no valid agent presence rows (0 ignored, 2 malformed)',
    }), DEFAULT_AGENT_PRESENCE_STATE)

    expect(view.label).toBe('Presence fallback')
    expect(view.detail).toBe('missing --:--Z · 2 malformed · Default ARDA state')
    expect(view.title).toContain('observed unknown')
    expect(view.className).toContain('--fallback')
  })

  it('shows an absent projection as missing with an unknown observation time', () => {
    expect(deriveBoardroomPresenceStatusView(undefined, DEFAULT_AGENT_PRESENCE_STATE)).toEqual({
      label: 'Presence fallback',
      detail: 'missing --:--Z · Default ARDA state',
      className: 'presence-ledger-status presence-ledger-status--fallback',
      title: 'Presence ledger status unavailable',
    })
  })
})
