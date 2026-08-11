import { describe, expect, it } from 'vitest'
import type { OnboardingSnapshot } from './tauri-core-compat'
import { summarizeFirstRunOrientation } from './first-run-orientation'

function snapshot(
  gateStatus: OnboardingSnapshot['gate_status'],
  overrides: Partial<OnboardingSnapshot> = {},
): OnboardingSnapshot {
  return {
    contract: 'arda.launcher.first-run.v1',
    generated_at_utc: '2026-08-11T12:00:00Z',
    gate_status: gateStatus,
    can_start_workbench: gateStatus === 'pass',
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
      message: 'Supported.',
    },
    prerequisites: { summary: { pass: 6 } },
    providers: { providers: [] },
    readiness: {
      gate_status: gateStatus,
      mode: 'read_only',
      mutation_policy: 'receipts_only_no_source_config_or_service_rewrites',
      summary: gateStatus === 'pass' ? { pass: 7 } : { pass: 6, fail: 1 },
      checks: gateStatus === 'pass' ? [] : [{
        check_id: 'provider',
        evidence: ['Manwe endpoint unavailable'],
        recommendation: 'Start Manwe, then refresh readiness.',
        severity: 'blocking',
        status: 'fail',
        title: 'Provider gateway',
      }],
      pass: [],
      warn: [],
    },
    servicePlan: {
      contract: 'arda.onboarding.service_plan.v1',
      generated_at_utc: '2026-08-11T12:00:00Z',
      profile: 'local',
      machine_role: 'workstation',
      gate_status: gateStatus,
      approval_contract_required: 'arda.onboarding.approval.v1',
      actions: [{
        action_id: 'install-service',
        action_type: 'install',
        title: 'Install root service',
        command_hint: 'arda install',
        target_path: null,
        requires_human_gate: true,
        description: 'Install the supervised service.',
        risk: 'changes service configuration',
      }],
    },
    guided: {
      steps: [{
        step_id: 'start',
        title: 'Start Workbench',
        status: 'ready',
        prompt: 'Open Workbench.',
        evidence: [],
        next_action: 'Open Workbench and attach a project.',
      }],
      next_actions: ['Open Workbench and attach a project.'],
    },
    recovery: [],
    optionalServices: [],
    ...overrides,
  }
}

describe('summarizeFirstRunOrientation', () => {
  it('states readiness, authority, evidence, blockers, and next action explicitly', () => {
    const summary = summarizeFirstRunOrientation(snapshot('pass'))

    expect(summary.systemState).toBe('Ready — required startup checks passed and Workbench can start.')
    expect(summary.canActNow).toContain('inspect readiness')
    expect(summary.approvalAuthority).toContain('Only you, the operator')
    expect(summary.approvalAuthority).toContain('No setup or project change runs automatically')
    expect(summary.evidenceQuality).toContain('7 passing readiness checks')
    expect(summary.executionBlockers).toBe('No current readiness blocker.')
    expect(summary.nextAction).toBe('Open Workbench and attach a project.')
  })

  it('names a blocking check and its recovery without implying execution authority', () => {
    const summary = summarizeFirstRunOrientation(snapshot('fail', { can_start_workbench: false }))

    expect(summary.systemState).toBe('Blocked — Workbench cannot start until failed readiness checks are resolved.')
    expect(summary.executionBlockers).toContain('Provider gateway')
    expect(summary.executionBlockers).toContain('Start Manwe, then refresh readiness.')
    expect(summary.nextAction).toBe('Start Manwe, then refresh readiness.')
    expect(summary.approvalAuthority).toContain('Only you, the operator')
  })

  it('fails closed on an unsupported profile', () => {
    const input = snapshot('pass', { can_start_workbench: true })
    input.compatibility.status = 'unsupported'
    input.compatibility.pretty_name = 'Ubuntu 24.04'

    const summary = summarizeFirstRunOrientation(input)

    expect(summary.systemState).toContain('Unsupported environment')
    expect(summary.executionBlockers).toContain('Ubuntu 24.04')
    expect(summary.nextAction).toContain('supported profile')
  })
})
