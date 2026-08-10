// sigil: REPAIR
import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  activeBoardroomSession,
  claimBoardroomSession,
  createEmptyBoardroomSessionRegistry,
  parseBoardroomSessionRegistry,
  refreshBoardroomSession,
  releaseBoardroomSession,
  type BoardroomSessionClaim,
} from './boardroomSessionRegistry'

describe('boardroom session registry', () => {
  const FIXED_NOW = '2026-07-30T12:00:00.000Z'

  function freezeTime(): Date {
    return new Date(FIXED_NOW)
  }

  beforeEach(() => {
    vi.useFakeTimers().setSystemTime(freezeTime())
  })

  it('parses a valid session registry and rejects unknown schema versions', () => {
    const registry = parseBoardroomSessionRegistry({
      schema_version: 'arda.boardroom.session_registry.v1',
      updated_at_utc: FIXED_NOW,
      sessions: {
        monitor_1: {
          slot_id: 'monitor_1',
          kind: 'monitor',
          owner: 'hermes-agent-001',
          opened_at_utc: FIXED_NOW,
          lease_expires_at_utc: '2026-07-30T12:10:00.000Z',
          metadata: { binding: 'routing_and_comms' },
        },
      },
    })
    expect(registry).not.toBeNull()
    expect(registry!.sessions.monitor_1.owner).toBe('hermes-agent-001')
    expect(parseBoardroomSessionRegistry({ schema_version: 'unknown', sessions: {} })).toBeNull()
  })

  it('claims, refreshes, and releases a monitor session under explicit TTLs', () => {
    let registry = createEmptyBoardroomSessionRegistry(FIXED_NOW)
    let claim = claimBoardroomSession(registry, 'monitor_1', 'monitor', 'hermes-agent-001', 5_000, { mode: 'native_window' }, FIXED_NOW)
    expect(claim.ok).toBe(true)
    expect(claim.session?.lease_expires_at_utc).toBe('2026-07-30T12:00:05.000Z')
    registry = claim.registry

    claim = refreshBoardroomSession(registry, 'monitor_1', 'hermes-agent-001', 10_000, FIXED_NOW)
    expect(claim.ok).toBe(true)
    expect(claim.session?.lease_expires_at_utc).toBe('2026-07-30T12:00:10.000Z')
    registry = claim.registry

    registry = releaseBoardroomSession(registry, 'monitor_1', 'hermes-agent-001', FIXED_NOW)
    expect(Object.keys(registry.sessions)).toHaveLength(0)
  })

  it('keeps sessions alive only while their lease is current', () => {
    let registry = createEmptyBoardroomSessionRegistry(FIXED_NOW)
    const claim = claimBoardroomSession(registry, 'monitor_1', 'monitor', 'hermes-agent-001', 5_000, {}, FIXED_NOW)
    registry = claim.registry
    const active = activeBoardroomSession(registry, 'monitor_1', '2026-07-30T12:00:06.000Z')
    expect(active).toBeNull()
  })

  it('rejects cross-owner refreshes and preserves the original session opener', () => {
    let registry = createEmptyBoardroomSessionRegistry(FIXED_NOW)
    const claim = claimBoardroomSession(registry, 'monitor_1', 'monitor', 'hermes-agent-001', 5_000, {}, FIXED_NOW)
    registry = claim.registry
    const rejected = refreshBoardroomSession(registry, 'monitor_1', 'other-agent', 5_000, FIXED_NOW)
    expect(rejected.ok).toBe(false)
    expect(rejected.session?.owner).toBe('hermes-agent-001')
    expect(rejected.session?.opened_at_utc).toBe(FIXED_NOW)
  })
})
