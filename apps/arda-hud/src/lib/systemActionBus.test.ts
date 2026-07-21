// sigil: REPAIR
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  executeSystemAction,
  getSystemActionAdapterPreset,
  getSystemActionCapabilityStatuses,
  getSystemActionDescriptors,
  setSystemActionAdapterPreset,
} from './systemActionBus'
import { safeTauriInvoke } from './tauriGuard'

vi.mock('./tauriGuard', () => ({
  safeTauriInvoke: vi.fn(),
}))

const mockedSafeTauriInvoke = vi.mocked(safeTauriInvoke)

const actionContext = {
  source: 'lounge' as const,
  persona: 'rache' as const,
  mood: 'success' as const,
}

const defaultPreset = getSystemActionAdapterPreset()

afterEach(() => {
  setSystemActionAdapterPreset(defaultPreset)
  mockedSafeTauriInvoke.mockReset()
  window.localStorage.clear()
})

describe('system action capability statuses', () => {
  it('exposes durable operator-facing capability fields for every action descriptor', () => {
    window.localStorage.clear()

    const descriptors = getSystemActionDescriptors()
    const statuses = getSystemActionCapabilityStatuses()

    expect(statuses).toHaveLength(descriptors.length)
    expect(statuses.map((status) => status.id)).toEqual(descriptors.map((descriptor) => descriptor.id))

    for (const status of statuses) {
      expect(status.name).toBeTruthy()
      expect(status.ownerSystem).toBeTruthy()
      expect(status.executor).toBeTruthy()
      expect(status.scheduleState).toMatch(/scheduled|manual_only|event_driven|not_scheduled/)
      expect(status.currentStatus).toMatch(/ready|blocked|not_observed/)
      expect(status.lastRun).toBe('not observed in this HUD profile')
      expect(status.nextRun).toBeTruthy()
      expect(status.requiredPermissions).toMatch(/read_only|dry_run|governed_mutation/)
      expect(status.governanceGate).toBeTruthy()
      expect(status.resultPath).toBeTruthy()
      expect(status.receiptPath).toBeTruthy()
      expect(status.failureReason).toBe('none observed')
      expect(status.relatedEvidence.length).toBeGreaterThan(0)
    }

    const governed = statuses.find((status) => status.id === 'arda.athena_promote_policy_ready')
    expect(governed).toMatchObject({
      currentStatus: 'blocked',
      manualRunAvailable: true,
      dryRunAvailable: true,
      governanceGate: 'human_review_required',
    })

    expect(statuses.map((status) => status.id)).toEqual(expect.arrayContaining([
      'arda.chronos_run_provider_checks',
      'arda.manwe_refresh_provider_intelligence',
      'arda.queue_preview_cleanup',
      'arda.athena_ingest_knowledge',
      'arda.setup_run_repair_flow',
      'arda.audit_run_repeated_audit',
    ]))
  })

  it('projects backend runtime receipts into the capability status contract', () => {
    window.localStorage.clear()

    const statuses = getSystemActionCapabilityStatuses({
      chronosRuntime: {
        generated_at_utc: '2026-05-27T12:00:00Z',
        status: 'baseline_active',
      },
      hadesNightlyOperations: {
        generated_at_utc: '2026-05-27T12:05:00Z',
        status: 'pass',
        artifacts: {
          organization_plan: 'data/hades/organization_plan_last.json',
          system_audit_summary: 'audit/system-audit-runs/latest/summary.json',
        },
        commands: {
          system_audit: {
            finished_at_utc: '2026-05-27T12:06:00Z',
            exit_code: 0,
          },
        },
      },
      setupConsoleReadiness: {
        generated_at_utc: '2026-05-27T12:07:00Z',
        gate_status: 'pass',
      },
      athenaRuntime: {
        generated_at_utc: '2026-05-27T12:08:00Z',
        status: 'ready',
      },
    })

    expect(statuses.find((status) => status.id === 'arda.chronos_run_provider_checks')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-27T12:00:00Z',
    })
    expect(statuses.find((status) => status.id === 'arda.audit_run_system_audit')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-27T12:06:00Z',
      receiptPath: 'audit/system-audit-runs/latest/summary.json',
      failureReason: 'none observed',
    })
    expect(statuses.find((status) => status.id === 'arda.athena_ingest_knowledge')).toMatchObject({
      currentStatus: 'blocked',
      lastRun: '2026-05-27T12:08:00Z',
    })
  })
})

describe('local CLI operator actions', () => {
  it('records human augmentation approval decisions through the Tauri local CLI adapter', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      success: true,
      content: 'approval recorded',
      error: null,
      path: '/var/home/mythos/Arda/core/state/human_augmentation_runtime.json',
    })

    const result = await executeSystemAction('approve_human_augmentation', {
      ...actionContext,
      payload: {
        arda_root: '/var/home/mythos/Arda',
        decision_class: 'arandur_mission_approval',
        command_signature: 'phase6g-approval-demo',
        approvers: 'aurelius,bacon',
        evidence: 'arda-hud,data/hades/lifecycle_review_queue.jsonl',
        note: 'Approved arandur mission packet.',
        status: 'approved',
      },
    })

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'Human augmentation approval recorded',
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('approve_human_augmentation_action', {
      numenorPath: '/var/home/mythos/Arda',
      decisionClass: 'arandur_mission_approval',
      commandSignature: 'phase6g-approval-demo',
      approvers: ['aurelius', 'bacon'],
      evidence: ['arda-hud', 'data/hades/lifecycle_review_queue.jsonl'],
      expiresAtUtc: null,
      note: 'Approved arandur mission packet.',
      status: 'approved',
    })
  })

  it('records CEO council sessions through the Tauri local CLI adapter', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      success: true,
      content: 'session recorded',
      error: null,
      path: '/var/home/mythos/Arda/data/council/sessions/session.json',
    })

    const result = await executeSystemAction('record_ceo_council_session', {
      ...actionContext,
      payload: {
        arda_root: '/var/home/mythos/Arda',
        objective: 'Approve bounded ARDA mission packet',
        loop_class: 'review_gate',
        decision_class: 'arandur_mission_approval',
        participants: 'prometheus,oracle,warden',
        proposals: 'approve packet,record evidence',
        objections: 'none',
        validators: 'oracle,warden',
        memory_lanes: 'mission,governance',
        memory_writes: 'mission: approved bounded packet\ngovernance: retained human gate evidence',
        synthesis: 'Approved with evidence.',
        triad_required: true,
        human_escalated: true,
        promoted_private_memory: false,
      },
    })

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'CEO council session recorded',
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('record_ceo_council_session_action', expect.objectContaining({
      numenorPath: '/var/home/mythos/Arda',
      objective: 'Approve bounded ARDA mission packet',
      loopClass: 'review_gate',
      decisionClass: 'arandur_mission_approval',
      participants: ['prometheus', 'oracle', 'warden'],
      proposals: ['approve packet', 'record evidence'],
      objections: ['none'],
      validatorsInvoked: ['oracle', 'warden'],
      memoryLanes: ['mission', 'governance'],
      memoryWrites: ['approved bounded packet', 'retained human gate evidence'],
      synthesis: 'Approved with evidence.',
      triadRequired: true,
      humanEscalated: true,
      promotedPrivateMemory: false,
      ingress: 'discord',
      ctoMode: 'hybrid',
      outcomeStatus: 'recorded',
    }))
  })

  it('invokes the CHRONOS provider check descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'CHRONOS provider checks refreshed',
      receiptPath: 'core/state/chronos_runtime.json',
      resultPath: 'core/state/chronos_runtime.json',
      generatedAt: '2026-05-28T05:30:00.000Z',
    })

    const result = await executeSystemAction('arda.chronos_run_provider_checks', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'CHRONOS provider checks refreshed',
      data: expect.objectContaining({
        receiptPath: 'core/state/chronos_runtime.json',
        resultPath: 'core/state/chronos_runtime.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_chronos_provider_checks', {
      actionId: 'arda.chronos_run_provider_checks',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.chronos_run_provider_checks')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T05:30:00.000Z',
      receiptPath: 'core/state/chronos_runtime.json',
      resultPath: 'core/state/chronos_runtime.json',
      failureReason: 'none observed',
    })
  })

  it('invokes the MANWE provider intelligence descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'MANWE provider intelligence refreshed (4 providers)',
      receiptPath: 'core/state/provider_intelligence.json',
      resultPath: 'core/state/provider_intelligence.json',
      generatedAt: '2026-05-28T06:20:00.000Z',
    })

    const result = await executeSystemAction('arda.manwe_refresh_provider_intelligence', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'MANWE provider intelligence refreshed (4 providers)',
      data: expect.objectContaining({
        receiptPath: 'core/state/provider_intelligence.json',
        resultPath: 'core/state/provider_intelligence.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_manwe_provider_intelligence_refresh', {
      actionId: 'arda.manwe_refresh_provider_intelligence',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.manwe_refresh_provider_intelligence')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T06:20:00.000Z',
      receiptPath: 'core/state/provider_intelligence.json',
      resultPath: 'core/state/provider_intelligence.json',
      failureReason: 'none observed',
    })
  })

  it('invokes the setup readiness descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'setup readiness refreshed',
      receiptPath: 'core/state/setup_console_readiness.json',
      resultPath: 'core/state/setup_console_readiness.json',
      generatedAt: '2026-05-27T12:14:00.000Z',
    })

    const result = await executeSystemAction('arda.setup_run_readiness_check', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'setup readiness refreshed',
      data: expect.objectContaining({
        receiptPath: 'core/state/setup_console_readiness.json',
        resultPath: 'core/state/setup_console_readiness.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_setup_readiness_check', {
      actionId: 'arda.setup_run_readiness_check',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.setup_run_readiness_check')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-27T12:14:00.000Z',
      receiptPath: 'core/state/setup_console_readiness.json',
      resultPath: 'core/state/setup_console_readiness.json',
      failureReason: 'none observed',
    })
  })

  it('invokes the queue cleanup preview descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'queue cleanup preview refreshed (0 queued tasks)',
      receiptPath: 'core/state/queue_summary.json',
      resultPath: 'core/state/queue_summary.json',
      generatedAt: '2026-05-28T05:38:06.000Z',
    })

    const result = await executeSystemAction('arda.queue_preview_cleanup', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'queue cleanup preview refreshed (0 queued tasks)',
      data: expect.objectContaining({
        receiptPath: 'core/state/queue_summary.json',
        resultPath: 'core/state/queue_summary.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_queue_cleanup_preview', {
      actionId: 'arda.queue_preview_cleanup',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.queue_preview_cleanup')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T05:38:06.000Z',
      receiptPath: 'core/state/queue_summary.json',
      resultPath: 'core/state/queue_summary.json',
      failureReason: 'none observed',
    })
  })

  it('invokes the ATHENA knowledge ingestion descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'ATHENA knowledge ingestion refreshed (3 files scanned)',
      receiptPath: 'core/state/knowledge_triage_registry.jsonl',
      resultPath: 'data/athena/human_ingestion_results.jsonl',
      generatedAt: '2026-05-28T06:05:00.000Z',
    })

    const result = await executeSystemAction('arda.athena_ingest_knowledge', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'ATHENA knowledge ingestion refreshed (3 files scanned)',
      data: expect.objectContaining({
        receiptPath: 'core/state/knowledge_triage_registry.jsonl',
        resultPath: 'data/athena/human_ingestion_results.jsonl',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_athena_knowledge_ingestion', {
      actionId: 'arda.athena_ingest_knowledge',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.athena_ingest_knowledge')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T06:05:00.000Z',
      receiptPath: 'core/state/knowledge_triage_registry.jsonl',
      resultPath: 'data/athena/human_ingestion_results.jsonl',
      failureReason: 'none observed',
    })
  })

  it('invokes the ATHENA digest refresh descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'ATHENA digest refreshed (25 entries)',
      receiptPath: 'core/state/athena_runtime.json',
      resultPath: 'data/athena/digest.jsonl',
      generatedAt: '2026-05-28T06:06:00.000Z',
    })

    const result = await executeSystemAction('arda.athena_refresh_digest', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'ATHENA digest refreshed (25 entries)',
      data: expect.objectContaining({
        receiptPath: 'core/state/athena_runtime.json',
        resultPath: 'data/athena/digest.jsonl',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_athena_digest_refresh', {
      actionId: 'arda.athena_refresh_digest',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.athena_refresh_digest')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T06:06:00.000Z',
      receiptPath: 'core/state/athena_runtime.json',
      resultPath: 'data/athena/digest.jsonl',
      failureReason: 'none observed',
    })
  })

  it('invokes the ATHENA policy readiness preview descriptor through the Tauri local CLI adapter without promotion mutation', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'ATHENA policy readiness preview refreshed (4 sources reviewed)',
      receiptPath: 'data/athena/policy_readiness.jsonl',
      resultPath: 'data/athena/policy_readiness.jsonl',
      generatedAt: '2026-05-28T06:08:00.000Z',
    })

    const result = await executeSystemAction('arda.athena_promote_policy_ready', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'ATHENA policy readiness preview refreshed (4 sources reviewed)',
      data: expect.objectContaining({
        receiptPath: 'data/athena/policy_readiness.jsonl',
        resultPath: 'data/athena/policy_readiness.jsonl',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_athena_policy_readiness_preview', {
      actionId: 'arda.athena_promote_policy_ready',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.athena_promote_policy_ready')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T06:08:00.000Z',
      receiptPath: 'data/athena/policy_readiness.jsonl',
      resultPath: 'data/athena/policy_readiness.jsonl',
      failureReason: 'none observed',
    })
  })

  it('invokes the HADES recurring maintenance descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'HADES recurring maintenance refreshed (pass)',
      receiptPath: 'core/state/hades_nightly_operations.json',
      resultPath: 'data/hades/organization_plan_last.json',
      generatedAt: '2026-05-28T06:07:00.000Z',
    })

    const result = await executeSystemAction('arda.hades_run_nightly', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'HADES recurring maintenance refreshed (pass)',
      data: expect.objectContaining({
        receiptPath: 'core/state/hades_nightly_operations.json',
        resultPath: 'data/hades/organization_plan_last.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_hades_recurring_maintenance', {
      actionId: 'arda.hades_run_nightly',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.hades_run_nightly')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T06:07:00.000Z',
      receiptPath: 'core/state/hades_nightly_operations.json',
      resultPath: 'data/hades/organization_plan_last.json',
      failureReason: 'none observed',
    })
  })

  it('invokes the repeated audit descriptor through the Tauri local CLI adapter and refreshes receipt-backed status', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'repeated audit preview refreshed (pass, 0 regressions)',
      receiptPath: 'core/state/repeated_audit_status.json',
      resultPath: 'audit/repeated-audit-runs/arda-hud-preview-last/summary.json',
      generatedAt: '2026-05-28T06:30:00.000Z',
    })

    const result = await executeSystemAction('arda.audit_run_repeated_audit', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'repeated audit preview refreshed (pass, 0 regressions)',
      data: expect.objectContaining({
        receiptPath: 'core/state/repeated_audit_status.json',
        resultPath: 'audit/repeated-audit-runs/arda-hud-preview-last/summary.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_repeated_audit_preview', {
      actionId: 'arda.audit_run_repeated_audit',
      source: 'lounge',
    })

    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.audit_run_repeated_audit')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T06:30:00.000Z',
      receiptPath: 'core/state/repeated_audit_status.json',
      resultPath: 'audit/repeated-audit-runs/arda-hud-preview-last/summary.json',
      failureReason: 'none observed',
    })
  })

  it('projects direct repeated audit runtime receipts into repeated audit capability status', () => {
    window.localStorage.clear()

    const statuses = getSystemActionCapabilityStatuses({
      repeatedAuditStatus: {
        generated_at_utc: '2026-05-28T06:31:00Z',
        gate_status: 'pass',
        outputs: {
          summary_json: 'audit/repeated-audit-runs/arda-hud-preview-last/summary.json',
        },
      },
    })

    expect(statuses.find((status) => status.id === 'arda.audit_run_repeated_audit')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T06:31:00Z',
      receiptPath: 'audit/repeated-audit-runs/arda-hud-preview-last/summary.json',
      resultPath: 'audit/repeated-audit-runs/arda-hud-preview-last/summary.json',
      failureReason: 'none observed',
    })
  })

  it('invokes the governed setup repair descriptor as a preflight-only Tauri local CLI action', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'setup repair preflight refreshed (warn; no repair mutations applied)',
      receiptPath: 'audit/setup-console-runs/arda-hud-repair-preflight-last/setup_console_readiness_receipt.json',
      resultPath: 'core/state/setup_repair_preflight.json',
      generatedAt: '2026-05-28T07:10:00.000Z',
    })

    const result = await executeSystemAction('arda.setup_run_repair_flow', actionContext)

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'setup repair preflight refreshed (warn; no repair mutations applied)',
      data: expect.objectContaining({
        receiptPath: 'audit/setup-console-runs/arda-hud-repair-preflight-last/setup_console_readiness_receipt.json',
        resultPath: 'core/state/setup_repair_preflight.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_setup_repair_preflight', {
      actionId: 'arda.setup_run_repair_flow',
      source: 'lounge',
    })
    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.setup_run_repair_flow')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T07:10:00.000Z',
      governanceGate: 'operator_approval_required_for_repair',
      receiptPath: 'audit/setup-console-runs/arda-hud-repair-preflight-last/setup_console_readiness_receipt.json',
      resultPath: 'core/state/setup_repair_preflight.json',
      failureReason: 'none observed',
    })
  })

  it('requires an explicit operator confirmation phrase before opening setup repair execution gate', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockResolvedValueOnce({
      ok: true,
      message: 'operator-confirmed setup repair execution gate opened (pass; repair mutations still disabled)',
      receiptPath: 'audit/setup-console-runs/arda-hud-repair-execution-gate-last/setup_console_readiness_receipt.json',
      resultPath: 'core/state/setup_repair_execution_gate.json',
      generatedAt: '2026-05-28T07:20:00.000Z',
    })

    const result = await executeSystemAction('arda.setup_run_repair_flow', {
      ...actionContext,
      payload: {
        repairExecutionConfirmation: 'RUN_SETUP_REPAIR_FLOW',
      },
    })

    expect(result).toMatchObject({
      ok: true,
      provider: 'tauri-local-cli',
      message: 'operator-confirmed setup repair execution gate opened (pass; repair mutations still disabled)',
      data: expect.objectContaining({
        receiptPath: 'audit/setup-console-runs/arda-hud-repair-execution-gate-last/setup_console_readiness_receipt.json',
        resultPath: 'core/state/setup_repair_execution_gate.json',
      }),
    })
    expect(mockedSafeTauriInvoke).toHaveBeenCalledTimes(1)
    expect(mockedSafeTauriInvoke).toHaveBeenCalledWith('run_setup_repair_execution_gate', {
      actionId: 'arda.setup_run_repair_flow',
      source: 'lounge',
      confirmationPhrase: 'RUN_SETUP_REPAIR_FLOW',
    })
    expect(getSystemActionCapabilityStatuses().find((status) => status.id === 'arda.setup_run_repair_flow')).toMatchObject({
      currentStatus: 'succeeded',
      lastRun: '2026-05-28T07:20:00.000Z',
      governanceGate: 'operator_approval_required_for_repair',
      receiptPath: 'audit/setup-console-runs/arda-hud-repair-execution-gate-last/setup_console_readiness_receipt.json',
      resultPath: 'core/state/setup_repair_execution_gate.json',
      failureReason: 'none observed',
    })
  })

  it('preserves command, path, and ENOENT details from local CLI failures', async () => {
    setSystemActionAdapterPreset('local_cli')
    const error = new Error('spawn arda-cli ENOENT') as Error & { code: string; path: string; command: string }
    error.code = 'ENOENT'
    error.path = '/var/home/mythos/Arda/scripts/missing.py'
    error.command = 'run_chronos_provider_checks'
    mockedSafeTauriInvoke.mockRejectedValueOnce(error)

    const result = await executeSystemAction('arda.chronos_run_provider_checks', actionContext)

    expect(result).toMatchObject({
      ok: false,
      provider: 'none',
    })
    expect(result.message).toContain('CHRONOS provider checks failed')
    expect(result.message).toContain('Error: spawn arda-cli ENOENT')
    expect(result.message).toContain('code=ENOENT')
    expect(result.message).toContain('path=/var/home/mythos/Arda/scripts/missing.py')
    expect(result.message).toContain('command=run_chronos_provider_checks')
  })

  it('preserves structured JSON/access failure details from local file actions', async () => {
    setSystemActionAdapterPreset('local_cli')
    mockedSafeTauriInvoke.mockRejectedValueOnce({
      name: 'ActionReceiptParseError',
      message: 'invalid JSON in receipt',
      code: 'JSON_PARSE',
      path: 'core/state/human_augmentation_runtime.json',
      stderr: 'Unexpected token }',
    })

    const result = await executeSystemAction('approve_human_augmentation', {
      ...actionContext,
      payload: {
        arda_root: '/var/home/mythos/Arda',
        decision_class: 'audit_batch_c',
      },
    })

    expect(result.ok).toBe(false)
    expect(result.message).toContain('ActionReceiptParseError')
    expect(result.message).toContain('invalid JSON in receipt')
    expect(result.message).toContain('code=JSON_PARSE')
    expect(result.message).toContain('path=core/state/human_augmentation_runtime.json')
    expect(result.message).toContain('stderr=Unexpected token }')
  })
})
