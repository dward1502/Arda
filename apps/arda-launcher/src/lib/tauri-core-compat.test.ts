import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import {
  invokeOnboardingSnapshot,
  invokeRegistryStatus,
  type ReadinessProjection,
  type RegistryStatusPayload,
  type ServicePlan,
} from './tauri-core-compat'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const invokeMock = vi.mocked(invoke)

const registry: RegistryStatusPayload = {
  loaded: true,
  schema_version: 'arda.contract_registry.v1',
  authority: 'registry',
  track_count: 3,
  gate_status: 'pass',
  tracks: [],
  checked_at_utc: '2026-07-29T00:00:00Z',
  error: null,
}

const readiness: ReadinessProjection = {
  gate_status: 'warn',
  mode: 'read_only',
  mutation_policy: 'receipts_only_no_source_config_or_service_rewrites',
  summary: { pass: 2, warn: 1 },
  pass: ['AGENTS.md'],
  warn: ['endpoint.missing_gates'],
}

const servicePlan: ServicePlan = {
  contract: 'arda.onboarding.service_plan.v1',
  generated_at_utc: '2026-07-29T00:00:00Z',
  profile: 'local',
  machine_role: 'workstation',
  gate_status: 'human_gated',
  approval_contract_required: 'arda.onboarding.approval.v1',
  actions: [
    {
      action_id: 'onboarding.set_manwe_endpoint',
      action_type: 'human_gate',
      title: 'Set MANWE endpoint',
      command_hint: 'set MANWE_BASE_URL',
      target_path: null,
      requires_human_gate: true,
      description: 'Coordinate endpoint discovery before mutation.',
      risk: 'human_gated',
    },
  ],
}

describe('Tauri onboarding command contract', () => {
  beforeEach(() => {
    invokeMock.mockReset()
  })

  it('invokes the typed registry command with an explicit root', async () => {
    invokeMock.mockResolvedValueOnce(registry)

    await expect(invokeRegistryStatus({ root: '/arda' })).resolves.toEqual(registry)
    expect(invokeMock).toHaveBeenCalledWith('registry_status', { root: '/arda' })
  })

  it('loads readiness and service plan through the real command names', async () => {
    invokeMock
      .mockResolvedValueOnce(readiness)
      .mockResolvedValueOnce(servicePlan)

    await expect(invokeOnboardingSnapshot({ root: '/arda' })).resolves.toEqual({
      readiness,
      servicePlan,
    })
    expect(invokeMock.mock.calls).toEqual([
      ['readiness_status', { root: '/arda' }],
      ['service_plan_status', { root: '/arda' }],
    ])
  })
})
