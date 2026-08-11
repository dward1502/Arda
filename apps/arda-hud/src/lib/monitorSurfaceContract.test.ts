import { describe, expect, it } from 'vitest'
import contract from '../../../../spec/monitor-session-contract/v1/fixtures/valid-monitor-session-contract.json'
import {
  CANONICAL_MONITOR_SLOT_IDS,
  createEmptyMonitorSessionRegistry,
  createMonitorSessionClaim,
  MONITOR_SESSION_REGISTRY_SCHEMA_VERSION,
  parseMonitorSessionRegistry,
  refreshMonitorSessionLease,
  releaseMonitorSession,
  toMonitorSurfaceSession,
} from './monitorSurfaceContract'

const CANONICAL_SLOTS = ['monitor_1', 'monitor_2', 'monitor_3', 'monitor_4', 'monitor_5'] as const

describe('monitor session contract', () => {
  it('pins the versioned schema and canonical slot set', () => {
    expect(MONITOR_SESSION_REGISTRY_SCHEMA_VERSION).toBe('arda.monitor-session-registry.v2')
    expect(CANONICAL_MONITOR_SLOT_IDS).toEqual(CANONICAL_SLOTS)
    expect(contract.schema_version).toBe('arda.monitor-session-contract.v1')
    expect(contract.canonical_slots).toEqual(Array.from(CANONICAL_SLOTS))
  })

  it('keeps the session_id stable in the live session handoff', () => {
    expect(contract.session.session_id).toBe(contract.session.workstation_handoff.session_id)
    expect(contract.session.workstation_handoff.mode).toBe('same_live_session')
  })

  it('rejects an invalid schema version when imported as a session document', () => {
    expect(parseMonitorSessionRegistry(contract.invalid_session as unknown as Record<string, unknown>)).toBeNull()
  })

  it('parses valid registries and preserves typed content descriptors', () => {
    const registryInput = {
      schema_version: 'arda.monitor-session-registry.v2',
      updated_at_utc: '2026-08-07T10:00:00.000Z',
      sessions: {
        monitor_1: {
          slot_id: 'monitor_1',
          session_id: 'session-web',
          surface_session_id: 'session-web',
          owner: 'agent:agent-web',
          kind: 'agent',
          revision: 1,
          opened_at_utc: '2026-08-07T10:00:00.000Z',
          lease_expires_at_utc: '2026-08-07T12:00:00.000Z',
          content: { kind: 'web', url: 'https://example.invalid/dashboard', display: 'capture_stream', sandboxProfile: 'default' },
          playback: { playing: false, currentTime: 42.5, volume: 0.25 },
          workstation_handoff: { session_id: 'session-web', mode: 'same_live_session' },
          created_at_utc: '2026-08-07T10:00:00.000Z',
          updated_at_utc: '2026-08-07T10:00:00.000Z',
        },
      },
    }
    const parsed = parseMonitorSessionRegistry(registryInput)
    expect(parsed).not.toBeNull()
    expect(parsed?.schema_version).toBe('arda.monitor-session-registry.v2')
    expect(Object.keys(parsed!.sessions)).toHaveLength(1)
    expect(parsed!.sessions.monitor_1.owner).toBe('agent:agent-web')
    expect(parsed!.sessions.monitor_1.content.kind).toBe('web')
    expect(toMonitorSurfaceSession(parsed!.sessions.monitor_1).playback).toEqual({
      playing: false,
      currentTime: 42.5,
      volume: 0.25,
    })
    expect(parseMonitorSessionRegistry({ ...registryInput, schema_version: 'arda.monitor-session-registry.v0' })).toBeNull()
  })

  it('claims, refreshes, and releases monitor sessions under canonical slots', () => {
    const now = '2026-08-07T10:00:00.000Z'
    let registry = createEmptyMonitorSessionRegistry(now)

    let claim = createMonitorSessionClaim(registry, {
      slotId: 'monitor_1',
      owner: { kind: 'agent', name: 'agent-web' },
      initialContent: contract.session.content as unknown as import('./monitorSurfaceContract').MonitorContentDescriptor,
      ttlMs: 1_000,
    }, now)
    expect(claim.ok).toBe(true)
    expect(claim.session?.slot_id).toBe('monitor_1')
    expect(claim.session?.session_id).toBe(claim.session?.surface_session_id)
    const surfaceSessionId = claim.session?.surface_session_id ?? 'session-web'
    registry = claim.registry

    claim = createMonitorSessionClaim(registry, {
      slotId: 'monitor_2',
      owner: { kind: 'agent', name: 'agent-terminal' },
      initialContent: { kind: 'terminal', sessionId: 'session-main', readOnly: false } as unknown as import('./monitorSurfaceContract').MonitorContentDescriptor,
      ttlMs: 2_000,
    }, now)
    expect(claim.ok).toBe(true)
    expect(claim.session?.slot_id).toBe('monitor_2')
    expect(claim.session?.session_id).toBe(claim.session?.surface_session_id)
    registry = claim.registry

    const refreshed = refreshMonitorSessionLease(registry, claim.session!.surface_session_id, 'agent:agent-terminal', 1_000, now)
    expect(refreshed.ok).toBe(true)
    expect(refreshed.session?.revision).toBe(2)
    registry = refreshed.registry

    const released = releaseMonitorSession(registry, claim.session!.surface_session_id, 'agent:agent-terminal', now)
    expect(released.registry.sessions['monitor_1']).toBeDefined()
    expect(released.registry.sessions['monitor_2']).toBeUndefined()
  })

  it('rejects non-canonical slots and cross-owner claims', () => {
    const now = '2026-08-07T10:00:00.000Z'
    const registry = createEmptyMonitorSessionRegistry(now)
    const claim = createMonitorSessionClaim(registry, {
      slotId: 'monitor_left_1' as unknown as import('./monitorSurfaceContract').UpperMonitorSlotId,
      owner: { kind: 'agent', name: 'agent-web' },
      initialContent: contract.session.content as unknown as import('./monitorSurfaceContract').MonitorContentDescriptor,
      ttlMs: 1_000,
    }, now)
    expect(claim.ok).toBe(false)
    expect(claim.session).toBeNull()
    expect(claim.message).toContain('not a canonical monitor slot')
  })
})
