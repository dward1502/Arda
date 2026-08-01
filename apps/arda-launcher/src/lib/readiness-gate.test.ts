import { describe, expect, it } from 'vitest'
import { evaluateReadinessGate } from './readiness-gate'
import type {
  OnboardingSnapshot,
  ReadinessProjection,
  RegistryStatusPayload,
} from './tauri-core-compat'

function registry(overrides: Partial<RegistryStatusPayload> = {}): RegistryStatusPayload {
  return {
    loaded: true,
    schema_version: '1',
    authority: 'fixture',
    track_count: 4,
    gate_status: 'pass',
    tracks: [],
    checked_at_utc: '2026-07-31T00:00:00Z',
    error: null,
    ...overrides,
  }
}

function readiness(
  gate_status: ReadinessProjection['gate_status'],
  summary: Record<string, number>,
): OnboardingSnapshot {
  return {
    readiness: {
      gate_status,
      mode: 'read-only',
      mutation_policy: 'human-gated',
      summary,
      checks: [],
      pass: [],
      warn: [],
    },
    servicePlan: null,
  }
}

describe('evaluateReadinessGate', () => {
  it('opens only when registry and setup readiness both pass', () => {
    expect(evaluateReadinessGate(registry(), readiness('pass', { pass: 7 }))).toEqual({
      registry: 'pass',
      statusLabel: 'Ready: 4 tracks and 7 setup checks verified',
      isReady: true,
    })
  })

  it('reports warnings without claiming readiness', () => {
    expect(evaluateReadinessGate(registry(), readiness('warn', { warn: 2 }))).toEqual({
      registry: 'warn',
      statusLabel: 'Readiness review: 2 warning(s)',
      isReady: false,
    })
  })

  it('reports missing readiness as a failure', () => {
    const snapshot: OnboardingSnapshot = { readiness: null, servicePlan: null }
    expect(evaluateReadinessGate(registry(), snapshot)).toEqual({
      registry: 'fail',
      statusLabel: 'Readiness blocked: 1 failed check(s)',
      isReady: false,
    })
  })

  it('preserves the registry load error', () => {
    expect(
      evaluateReadinessGate(
        registry({ loaded: false, gate_status: 'fail', error: 'registry unreadable' }),
        readiness('pass', { pass: 7 }),
      ),
    ).toEqual({
      registry: 'fail',
      statusLabel: 'registry unreadable',
      isReady: false,
    })
  })
})
