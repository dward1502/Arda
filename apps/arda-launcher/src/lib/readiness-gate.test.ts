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
    contract: 'arda.launcher.first-run.v1',
    generated_at_utc: '2026-08-04T00:00:00Z',
    gate_status,
    can_start_workbench: gate_status === 'pass',
    mutation_policy: 'explicit_approval_and_receipt_required',
    profile: 'local',
    machine_role: 'workstation',
    compatibility: {
      status: 'supported',
      profile_id: 'bluefin-lts-10-x86_64',
      supported_profile: 'bluefin-lts-10-x86_64',
      architecture: 'x86_64',
      os_id: 'centos',
      version_id: '10',
      pretty_name: 'Bluefin LTS 10',
      message: 'supported',
    },
    prerequisites: { summary: { pass: 4 } },
    providers: { providers: [] },
    readiness: {
      gate_status,
      mode: 'read-only',
      mutation_policy: 'human-gated',
      summary,
      checks: [],
      pass: [],
      warn: [],
    },
    servicePlan: {
      contract: 'arda.onboarding.service_plan.v1',
      generated_at_utc: '2026-08-04T00:00:00Z',
      profile: 'local',
      machine_role: 'workstation',
      gate_status: 'human_gated',
      approval_contract_required: 'arda.onboarding.approval.v1',
      actions: [],
    },
    guided: { steps: [], next_actions: [] },
    recovery: [],
    optionalServices: [],
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

  it('reports failed readiness as a failure', () => {
    expect(evaluateReadinessGate(registry(), readiness('fail', { fail: 1 }))).toEqual({
      registry: 'fail',
      statusLabel: 'Readiness blocked: 1 failed check(s)',
      isReady: false,
    })
  })

  it('blocks unsupported profiles before readiness can pass', () => {
    const snapshot = readiness('pass', { pass: 7 })
    snapshot.compatibility.status = 'unsupported'
    snapshot.compatibility.pretty_name = 'Ubuntu 24.04'
    expect(evaluateReadinessGate(registry(), snapshot)).toEqual({
      registry: 'fail',
      statusLabel: 'Unsupported profile: Ubuntu 24.04',
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
