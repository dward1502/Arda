// sigil: REPAIR
import { postAction } from './weathertop'
import { safeTauriInvoke } from './tauriGuard'
import { parseJsonOrNull } from './jsonParse'
import { localStorageOrNull } from './settingsLayout'
import type { AvatarMood, AvatarPersona } from './avatarPersona'

export type SystemActionId =
  | 'status_report'
  | 'purge_stale'
  | 'deploy_last_script'
  | 'chill_mode'
  | 'queue_next'
  | 'activate_plan'
  | 'approve_human_augmentation'
  | 'review_arandur_recommendation'
  | 'record_ceo_council_session'
  | 'arda.chronos_run_provider_checks'
  | 'arda.manwe_refresh_provider_intelligence'
  | 'arda.queue_preview_cleanup'
  | 'arda.queue_capture_pivot'
  | 'arda.hades_run_nightly'
  | 'arda.hades_preview_organization_plan'
  | 'arda.hades_run_link_check'
  | 'arda.athena_ingest_knowledge'
  | 'arda.athena_refresh_digest'
  | 'arda.athena_promote_policy_ready'
  | 'arda.audit_run_system_audit'
  | 'arda.audit_run_repeated_audit'
  | 'arda.setup_run_readiness_check'
  | 'arda.setup_run_repair_flow'

export type SystemActionRiskLevel = 'read_only' | 'dry_run' | 'governed_mutation'
export type SystemActionScheduleState = 'scheduled' | 'manual_only' | 'event_driven' | 'not_scheduled'
export type SystemActionCurrentStatus = 'not_observed' | 'ready' | 'running' | 'succeeded' | 'failed' | 'blocked'

export interface SystemActionDescriptor {
  id: SystemActionId
  label: string
  owner: string
  executor: string
  purpose: string
  riskLevel: SystemActionRiskLevel
  automationEligible: boolean
  userTriggerEligible: boolean
  scheduleState: SystemActionScheduleState
  nextRun?: string
  governanceGate: string
  dryRunSupported: boolean
  resultPath?: string
  receiptPath?: string
  relatedEvidence: string[]
}

export interface SystemActionCapabilityStatus {
  id: SystemActionId
  name: string
  purpose: string
  ownerSystem: string
  executor: string
  scheduleState: SystemActionScheduleState
  lastRun: string
  nextRun: string
  currentStatus: SystemActionCurrentStatus
  requiredPermissions: string
  governanceGate: string
  manualRunAvailable: boolean
  dryRunAvailable: boolean
  resultPath: string
  receiptPath: string
  failureReason: string
  relatedEvidence: string[]
}

export type SystemActionRuntimeReceipts = Partial<Record<
  | 'chronosRuntime'
  | 'providerIntelligence'
  | 'queueSummary'
  | 'setupConsoleReadiness'
  | 'setupRepairPreflight'
  | 'repeatedAuditStatus'
  | 'hadesNightlyOperations'
  | 'athenaRuntime'
  | 'knowledgeTriage',
  unknown
>>

interface PersistedSystemActionExecution {
  action: SystemActionId
  ok: boolean
  provider: string
  message: string
  at: number
  durationMs: number
  generatedAt?: string
  resultPath?: string
  receiptPath?: string
}

export interface SystemActionContext {
  source: 'lounge' | 'voice' | 'streamdeck' | 'external'
  persona: AvatarPersona
  mood: AvatarMood
  payload?: Record<string, unknown>
}

export interface SystemActionResult {
  ok: boolean
  provider: string
  message: string
  data?: unknown
}

export interface SystemActionExecutionEvent {
  action: SystemActionId
  context: SystemActionContext
  result: SystemActionResult
  at: number
  durationMs: number
}

export interface SystemActionAdapter {
  id: string
  presets?: AdapterPresetId[]
  canHandle: (action: SystemActionId, context: SystemActionContext) => boolean | Promise<boolean>
  execute: (action: SystemActionId, context: SystemActionContext) => Promise<SystemActionResult>
}

export type AdapterPresetId = 'auto' | 'webhook' | 'discord_bridge' | 'local_cli'

declare global {
  interface Window {
    ardaActionAdapter?: {
      execute: (action: SystemActionId, context: SystemActionContext) => Promise<SystemActionResult>
    }
  }
}

const runtimeAdapters: SystemActionAdapter[] = []
let activePreset: AdapterPresetId = 'auto'
const ACTION_EXECUTION_LEDGER_KEY = 'arda.system_action.execution_ledger.v1'

function fallbackDescriptor(action: SystemActionId): SystemActionDescriptor {
  return {
    id: action,
    label: action,
    owner: 'unknown',
    executor: 'unknown',
    purpose: 'unknown',
    riskLevel: 'read_only',
    automationEligible: false,
    userTriggerEligible: false,
    scheduleState: 'not_scheduled',
    governanceGate: 'unknown',
    dryRunSupported: false,
    relatedEvidence: [],
  }
}

function descriptorForAction(action: SystemActionId): SystemActionDescriptor {
  return SYSTEM_ACTION_DESCRIPTORS.find((descriptor) => descriptor.id === action) ?? fallbackDescriptor(action)
}

const SYSTEM_ACTION_DESCRIPTORS: SystemActionDescriptor[] = [
  {
    id: 'arda.chronos_run_provider_checks',
    label: 'Run CHRONOS Provider Checks',
    owner: 'CHRONOS',
    executor: 'arda-chronos runtime projection',
    purpose: 'Refresh feed health and provider/runtime anomaly checks for the operating surface.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'scheduled',
    nextRun: 'next Chronos projection window',
    governanceGate: 'runtime_projection_read_only',
    dryRunSupported: false,
    resultPath: 'core/state/chronos_runtime.json',
    receiptPath: 'core/state/chronos_runtime.json',
    relatedEvidence: ['core/state/manwe_router.json', 'core/state/provider_intelligence.json'],
  },
  {
    id: 'arda.manwe_refresh_provider_intelligence',
    label: 'Refresh Provider Intelligence',
    owner: 'MANWE',
    executor: 'scripts/refresh_provider_intelligence.py',
    purpose: 'Refresh provider/model availability intelligence used by routing and readiness surfaces.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'scheduled',
    nextRun: 'next provider intelligence refresh',
    governanceGate: 'provider_metadata_refresh_only',
    dryRunSupported: false,
    resultPath: 'core/state/provider_intelligence.json',
    receiptPath: 'core/state/provider_intelligence.json',
    relatedEvidence: ['core/state/manwe_router.json', 'core/state/provider_token_usage.json'],
  },
  {
    id: 'arda.queue_preview_cleanup',
    label: 'Preview Queue Cleanup',
    owner: 'HADES',
    executor: 'arda-cli export autonomy-resume',
    purpose: 'Summarize queued/completed/cancelled project work before any cleanup or lifecycle action.',
    riskLevel: 'dry_run',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'event_driven',
    nextRun: 'on queue pressure or operator request',
    governanceGate: 'queue_cleanup_preview_only',
    dryRunSupported: true,
    resultPath: 'core/state/queue_summary.json',
    receiptPath: 'core/state/queue_summary.json',
    relatedEvidence: ['core/projects/tasks/queue.jsonl'],
  },
  {
    id: 'arda.queue_capture_pivot',
    label: 'Capture Task Pivot',
    owner: 'PROMETHEUS',
    executor: 'arda-cli utility task-pivot',
    purpose: 'Record meaningful project pivots into the durable task queue with owner, priority, scope, and glyph metadata.',
    riskLevel: 'governed_mutation',
    automationEligible: false,
    userTriggerEligible: true,
    scheduleState: 'manual_only',
    nextRun: 'manual operator action',
    governanceGate: 'operator_intent_required',
    dryRunSupported: true,
    resultPath: 'core/projects/tasks/queue.jsonl',
    receiptPath: 'core/state/queue_summary.json',
    relatedEvidence: ['core/projects/tasks/queue.jsonl'],
  },
  {
    id: 'arda.hades_run_nightly',
    label: 'Run HADES Nightly',
    owner: 'HADES',
    executor: 'scripts/hades_nightly_operations.sh',
    purpose: 'Execute nightly audit/setup/organization maintenance receipt generation.',
    riskLevel: 'dry_run',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'scheduled',
    nextRun: 'next nightly operations window',
    governanceGate: 'audit_receipts_only_no_source_config_service_or_queue_mutation',
    dryRunSupported: true,
    resultPath: 'data/hades/organization_plan_last.json',
    receiptPath: 'core/state/hades_nightly_operations.json',
    relatedEvidence: ['data/hades/organization_plan_last.json', 'data/hades/markdown_link_check_last.md'],
  },
  {
    id: 'arda.hades_preview_organization_plan',
    label: 'Preview Organization Plan',
    owner: 'HADES',
    executor: 'scripts/hades_organization_maintenance.sh --plan-only',
    purpose: 'Review duplicate, stale, generated, and lifecycle file candidates before mutation.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'scheduled',
    nextRun: 'with HADES organization maintenance',
    governanceGate: 'review_only',
    dryRunSupported: true,
    resultPath: 'data/hades/organization_plan_last.json',
    receiptPath: 'data/hades/organization_plan_last.json',
    relatedEvidence: ['core/state/hades_nightly_operations.json'],
  },
  {
    id: 'arda.hades_run_link_check',
    label: 'Run Link Check',
    owner: 'HADES',
    executor: 'scripts/hades_organization_maintenance.sh --link-check',
    purpose: 'Refresh markdown local-link evidence for documentation and file lifecycle review.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'scheduled',
    nextRun: 'with HADES organization maintenance',
    governanceGate: 'review_only',
    dryRunSupported: true,
    resultPath: 'data/hades/markdown_link_check_last.md',
    receiptPath: 'data/hades/markdown_link_check_last.md',
    relatedEvidence: ['core/state/hades_nightly_operations.json'],
  },
  {
    id: 'arda.athena_ingest_knowledge',
    label: 'Ingest Knowledge Notes',
    owner: 'ATHENA',
    executor: 'ATHENA knowledge triage registry',
    purpose: 'Classify new human/knowledge inputs into digest, deep graph, policy readiness, or task-promotion candidates.',
    riskLevel: 'governed_mutation',
    automationEligible: false,
    userTriggerEligible: true,
    scheduleState: 'event_driven',
    nextRun: 'when knowledge intake changes',
    governanceGate: 'human_review_required_for_promotion',
    dryRunSupported: true,
    resultPath: 'core/state/knowledge_triage_registry.jsonl',
    receiptPath: 'core/state/knowledge_triage_registry.jsonl',
    relatedEvidence: ['human/', 'data/athena/digest.jsonl'],
  },
  {
    id: 'arda.athena_refresh_digest',
    label: 'Refresh ATHENA Digest',
    owner: 'ATHENA',
    executor: 'arda-athena runtime projection',
    purpose: 'Update digest, deep graph, policy readiness, and source pressure projections.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'scheduled',
    nextRun: 'next ATHENA digest refresh',
    governanceGate: 'knowledge_refresh_only',
    dryRunSupported: false,
    resultPath: 'data/athena/digest.jsonl',
    receiptPath: 'core/state/athena_runtime.json',
    relatedEvidence: ['data/athena/deep_graph.jsonl', 'data/athena/policy_readiness.jsonl'],
  },
  {
    id: 'arda.athena_promote_policy_ready',
    label: 'Promote Policy-Ready Knowledge',
    owner: 'ATHENA',
    executor: 'ATHENA policy readiness queue',
    purpose: 'Prepare policy-ready knowledge for governed human review and promotion.',
    riskLevel: 'governed_mutation',
    automationEligible: false,
    userTriggerEligible: true,
    scheduleState: 'manual_only',
    nextRun: 'manual operator approval',
    governanceGate: 'human_review_required',
    dryRunSupported: true,
    resultPath: 'data/athena/policy_readiness.jsonl',
    receiptPath: 'data/athena/policy_readiness.jsonl',
    relatedEvidence: ['core/state/knowledge_triage_registry.jsonl'],
  },
  {
    id: 'arda.audit_run_system_audit',
    label: 'Run System Audit',
    owner: 'WARDEN',
    executor: 'scripts/audit/system_audit.py',
    purpose: 'Refresh system audit evidence and UX/operator readiness scores.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'event_driven',
    nextRun: 'on operator/audit request',
    governanceGate: 'audit_receipts_only',
    dryRunSupported: false,
    resultPath: 'audit/system-audit-runs',
    receiptPath: 'audit/system-audit-runs',
    relatedEvidence: ['audit/'],
  },
  {
    id: 'arda.audit_run_repeated_audit',
    label: 'Run Repeated Audit',
    owner: 'CHRONOS',
    executor: 'scripts/audit/repeated_audit.py',
    purpose: 'Run repeated audit regression checks and candidate-task gate receipts.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'scheduled',
    nextRun: 'next repeated audit window',
    governanceGate: 'audit_receipts_only',
    dryRunSupported: false,
    resultPath: 'audit/repeated-audit-runs',
    receiptPath: 'core/state/repeated_audit_status.json',
    relatedEvidence: ['core/state/hades_nightly_operations.json'],
  },
  {
    id: 'arda.setup_run_readiness_check',
    label: 'Run Setup Readiness Check',
    owner: 'CLI',
    executor: 'arda-cli setup readiness surface',
    purpose: 'Refresh onboarding, local service, config, and portability readiness.',
    riskLevel: 'read_only',
    automationEligible: true,
    userTriggerEligible: true,
    scheduleState: 'event_driven',
    nextRun: 'on setup/onboarding request',
    governanceGate: 'readiness_receipt_only',
    dryRunSupported: false,
    resultPath: 'core/state/setup_console_readiness.json',
    receiptPath: 'core/state/setup_console_readiness.json',
    relatedEvidence: ['config/', 'core/state/operator_runtime_status.json'],
  },
  {
    id: 'arda.setup_run_repair_flow',
    label: 'Run Setup Repair Flow',
    owner: 'CLI',
    executor: 'setup repair flow / operator-guided remediation',
    purpose: 'Run a governed setup-repair preflight by default, and open the repair execution gate only when an operator supplies the explicit confirmation phrase; repair mutations remain disabled until that confirmation path is present.',
    riskLevel: 'governed_mutation',
    automationEligible: false,
    userTriggerEligible: true,
    scheduleState: 'manual_only',
    nextRun: 'manual operator approval with RUN_SETUP_REPAIR_FLOW confirmation',
    governanceGate: 'operator_approval_required_for_repair',
    dryRunSupported: true,
    resultPath: 'core/state/setup_repair_preflight.json',
    receiptPath: 'audit/setup-console-runs/arda-hud-repair-preflight-last/setup_console_readiness_receipt.json',
    relatedEvidence: ['core/state/setup_console_readiness.json'],
  }
]

function readPersistedExecutionLedger(): Partial<Record<SystemActionId, PersistedSystemActionExecution>> {
  const storage = localStorageOrNull()
  if (!storage) return {}
  try {
    const raw = storage.getItem(ACTION_EXECUTION_LEDGER_KEY)
    if (!raw) return {}
    const parsed = parseJsonOrNull<unknown>(raw)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {}
    return Object.fromEntries(
      Object.entries(parsed).filter((entry): entry is [string, unknown] => typeof entry[0] === 'string'),
    ) as Partial<Record<SystemActionId, PersistedSystemActionExecution>>
  } catch {
    return {}
  }
}

function writePersistedExecution(event: SystemActionExecutionEvent): void {
  const storage = localStorageOrNull()
  if (!storage) return
  try {
    const ledger = readPersistedExecutionLedger()
    ledger[event.action] = {
      action: event.action,
      ok: event.result.ok,
      provider: event.result.provider,
      message: event.result.message,
      at: event.at,
      durationMs: event.durationMs,
      generatedAt: asString(resultRecord(event.result)?.generatedAt),
      resultPath: asString(resultRecord(event.result)?.resultPath),
      receiptPath: asString(resultRecord(event.result)?.receiptPath),
    }
    storage.setItem(ACTION_EXECUTION_LEDGER_KEY, JSON.stringify(ledger))
  } catch {
    // Keep action execution non-blocking if storage is unavailable or full.
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function asString(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

function resultRecord(result: SystemActionResult): Record<string, unknown> | null {
  return asRecord(result.data)
}

function nestedRecord(record: Record<string, unknown>, key: string): Record<string, unknown> | null {
  return asRecord(record[key])
}

function commandReceipt(record: Record<string, unknown> | null, command: string): Record<string, unknown> | null {
  if (!record) return null
  const commands = nestedRecord(record, 'commands')
  return commands ? asRecord(commands[command]) : null
}

function statusFromGate(value: unknown): SystemActionCurrentStatus | undefined {
  const status = asString(value)?.toLowerCase()
  if (!status) return undefined
  if (['pass', 'passed', 'ok', 'success', 'succeeded', 'baseline_active', 'ready'].includes(status)) return 'succeeded'
  if (['fail', 'failed', 'error'].includes(status)) return 'failed'
  if (['running', 'in_progress'].includes(status)) return 'running'
  if (['blocked', 'hold'].includes(status)) return 'blocked'
  return undefined
}

function generatedAt(record: Record<string, unknown> | null): string | undefined {
  if (!record) return undefined
  return asString(record.generated_at_utc) ?? asString(record.generated_at) ?? asString(record.finished_at_utc) ?? asString(record.started_at_utc)
}

function backendReceiptForDescriptor(
  descriptor: SystemActionDescriptor,
  receipts: SystemActionRuntimeReceipts | undefined,
): Pick<SystemActionCapabilityStatus, 'lastRun' | 'currentStatus' | 'failureReason' | 'receiptPath' | 'resultPath'> | null {
  if (!receipts) return null

  const chronosRuntime = asRecord(receipts.chronosRuntime)
  const providerIntelligence = asRecord(receipts.providerIntelligence)
  const queueSummary = asRecord(receipts.queueSummary)
  const setupConsoleReadiness = asRecord(receipts.setupConsoleReadiness)
  const setupRepairPreflight = asRecord(receipts.setupRepairPreflight)
  const repeatedAuditStatus = asRecord(receipts.repeatedAuditStatus)
  const hadesNightlyOperations = asRecord(receipts.hadesNightlyOperations)
  const athenaRuntime = asRecord(receipts.athenaRuntime)

  let source: Record<string, unknown> | null = null
  let command: Record<string, unknown> | null = null
  let resultPath = descriptor.resultPath ?? 'not declared'
  let receiptPath = descriptor.receiptPath ?? 'not declared'
  let currentStatus: SystemActionCurrentStatus | undefined
  let failureReason = 'none observed'

  switch (descriptor.id) {
    case 'arda.chronos_run_provider_checks':
      source = chronosRuntime
      currentStatus = statusFromGate(source?.status) ?? (source ? 'succeeded' : undefined)
      break
    case 'arda.manwe_refresh_provider_intelligence':
      source = providerIntelligence
      currentStatus = source ? 'succeeded' : undefined
      break
    case 'arda.queue_preview_cleanup':
    case 'arda.queue_capture_pivot':
      source = queueSummary
      currentStatus = source ? (descriptor.riskLevel === 'governed_mutation' ? 'blocked' : 'succeeded') : undefined
      break
    case 'arda.hades_run_nightly':
      source = hadesNightlyOperations
      currentStatus = statusFromGate(source?.status)
      break
    case 'arda.hades_preview_organization_plan':
      source = hadesNightlyOperations
      command = commandReceipt(source, 'hades_organization_maintenance')
      currentStatus = statusFromGate(source?.status)
      resultPath = asString(nestedRecord(source ?? {}, 'artifacts')?.organization_plan) ?? resultPath
      receiptPath = resultPath
      break
    case 'arda.hades_run_link_check':
      source = hadesNightlyOperations
      command = commandReceipt(source, 'hades_organization_maintenance')
      currentStatus = statusFromGate(source?.status)
      resultPath = asString(nestedRecord(source ?? {}, 'artifacts')?.markdown_link_check) ?? resultPath
      receiptPath = resultPath
      break
    case 'arda.audit_run_system_audit':
      source = hadesNightlyOperations
      command = commandReceipt(source, 'system_audit')
      currentStatus = statusFromGate(source?.status)
      resultPath = asString(nestedRecord(source ?? {}, 'artifacts')?.system_audit_summary) ?? resultPath
      receiptPath = resultPath
      break
    case 'arda.audit_run_repeated_audit':
      source = repeatedAuditStatus ?? hadesNightlyOperations
      command = repeatedAuditStatus ? null : commandReceipt(source, 'repeated_audit')
      currentStatus = repeatedAuditStatus
        ? statusFromGate(source?.gate_status) ?? statusFromGate(source?.status)
        : statusFromGate(source?.status)
      resultPath = asString(nestedRecord(source ?? {}, 'outputs')?.summary_json)
        ?? asString(nestedRecord(source ?? {}, 'artifacts')?.repeated_audit_summary)
        ?? resultPath
      receiptPath = resultPath
      break
    case 'arda.setup_run_readiness_check':
      source = setupConsoleReadiness
      command = commandReceipt(hadesNightlyOperations, 'setup_console_readiness')
      currentStatus = statusFromGate(source?.gate_status) ?? statusFromGate(command?.exit_code === 0 ? 'pass' : undefined)
      receiptPath = asString(command?.receipt) ?? receiptPath
      break
    case 'arda.setup_run_repair_flow':
      source = setupRepairPreflight ?? setupConsoleReadiness
      currentStatus = setupRepairPreflight
        ? statusFromGate(source?.gate_status) ?? statusFromGate(source?.status)
        : source
          ? 'blocked'
          : undefined
      break
    case 'arda.athena_refresh_digest':
      source = athenaRuntime
      currentStatus = source ? (statusFromGate(source.status) ?? 'succeeded') : undefined
      break
    case 'arda.athena_ingest_knowledge':
    case 'arda.athena_promote_policy_ready':
      source = athenaRuntime ?? asRecord(receipts.knowledgeTriage)
      currentStatus = source ? 'blocked' : undefined
      break
    default:
      break
  }

  if (!source && !command) return null
  if (command) {
    currentStatus = command.exit_code === 0 ? 'succeeded' : command.exit_code === undefined ? currentStatus : 'failed'
    failureReason = command.exit_code === 0 ? 'none observed' : asString(command.stderr_tail) ?? `exit code ${String(command.exit_code)}`
  }

  return {
    lastRun: generatedAt(command) ?? generatedAt(source) ?? 'observed without timestamp',
    currentStatus: currentStatus ?? statusForDescriptor(descriptor, undefined),
    failureReason,
    resultPath,
    receiptPath,
  }
}

function statusForDescriptor(
  descriptor: SystemActionDescriptor,
  execution: PersistedSystemActionExecution | undefined,
): SystemActionCurrentStatus {
  if (execution) {
    if (execution.ok) return 'succeeded'
    if (descriptor.riskLevel === 'governed_mutation' && /governed|approval|required|disabled/i.test(execution.message)) {
      return 'blocked'
    }
    return 'failed'
  }
  if (descriptor.riskLevel === 'governed_mutation') return 'blocked'
  if (descriptor.userTriggerEligible || descriptor.automationEligible) return 'ready'
  return 'not_observed'
}

export function getSystemActionCapabilityStatuses(receipts?: SystemActionRuntimeReceipts): SystemActionCapabilityStatus[] {
  const ledger = readPersistedExecutionLedger()
  return SYSTEM_ACTION_DESCRIPTORS.map((descriptor) => {
    const execution = ledger[descriptor.id]
    const backend = backendReceiptForDescriptor(descriptor, receipts)
    return {
      id: descriptor.id,
      name: descriptor.label,
      purpose: descriptor.purpose,
      ownerSystem: descriptor.owner,
      executor: descriptor.executor,
      scheduleState: descriptor.scheduleState,
      lastRun: execution ? execution.generatedAt ?? new Date(execution.at).toISOString() : backend?.lastRun ?? 'not observed in this HUD profile',
      nextRun: descriptor.nextRun ?? (descriptor.scheduleState === 'manual_only' ? 'manual only' : 'not declared'),
      currentStatus: statusForDescriptor(descriptor, execution) === 'failed' || execution ? statusForDescriptor(descriptor, execution) : backend?.currentStatus ?? statusForDescriptor(descriptor, execution),
      requiredPermissions: descriptor.riskLevel,
      governanceGate: descriptor.governanceGate,
      manualRunAvailable: descriptor.userTriggerEligible,
      dryRunAvailable: descriptor.dryRunSupported,
      resultPath: execution?.resultPath ?? backend?.resultPath ?? descriptor.resultPath ?? 'not declared',
      receiptPath: execution?.receiptPath ?? backend?.receiptPath ?? descriptor.receiptPath ?? 'not declared',
      failureReason: execution && !execution.ok ? execution.message : backend?.failureReason ?? 'none observed',
      relatedEvidence: descriptor.relatedEvidence,
    }
  })
}

export function setSystemActionAdapterPreset(preset: AdapterPresetId): void {
  activePreset = preset
}

export function getSystemActionAdapterPreset(): AdapterPresetId {
  return activePreset
}

export function registerSystemActionAdapter(adapter: SystemActionAdapter): () => void {
  runtimeAdapters.unshift(adapter)
  return () => {
    const index = runtimeAdapters.findIndex((item) => item.id === adapter.id)
    if (index >= 0) {
      runtimeAdapters.splice(index, 1)
    }
  }
}

export function getSystemActionDescriptors(): SystemActionDescriptor[] {
  return [...SYSTEM_ACTION_DESCRIPTORS]
}

export function getSystemActionBusStatus(): {
  runtimeAdapterCount: number
  hasWindowAdapter: boolean
  defaultProviders: string[]
  activePreset: AdapterPresetId
  availablePresets: AdapterPresetId[]
} {
  return {
    runtimeAdapterCount: runtimeAdapters.length,
    hasWindowAdapter: typeof window !== 'undefined' && typeof window.ardaActionAdapter?.execute === 'function',
    defaultProviders: ['weathertop-http', 'browser-event'],
    activePreset,
    availablePresets: ['auto', 'webhook', 'discord_bridge', 'local_cli'],
  }
}

function emitExecutionEvent(event: SystemActionExecutionEvent) {
  writePersistedExecution(event)
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent<SystemActionExecutionEvent>('arda:system-action-result', { detail: event }))
}

const windowAdapter: SystemActionAdapter = {
  id: 'window-arda-action-adapter',
  presets: ['auto', 'local_cli'],
  canHandle: () => typeof window !== 'undefined' && typeof window.ardaActionAdapter?.execute === 'function',
  execute: async (action, context) => {
    if (!window.ardaActionAdapter?.execute) {
      return { ok: false, provider: 'window-arda-action-adapter', message: 'No window adapter registered' }
    }
    return window.ardaActionAdapter.execute(action, context)
  },
}

const weathertopAdapter: SystemActionAdapter = {
  id: 'weathertop-http',
  presets: ['auto', 'webhook'],
  canHandle: () => true,
  execute: async (action, context) => {
    const ok = await postAction({
      type: 'system_action',
      target: action,
      payload: {
        source: context.source,
        persona: context.persona,
        mood: context.mood,
        ...(context.payload ?? {}),
      },
    })

    if (!ok) {
      return { ok: false, provider: 'weathertop-http', message: 'Weathertop action endpoint unavailable' }
    }

    return { ok: true, provider: 'weathertop-http', message: `Action sent: ${action}` }
  },
}

export interface LocalFileActionInvokeResult {
  success: boolean
  content?: string | null
  error?: string | null
  path: string
}

export interface LocalActionInvokeResult {
  ok?: boolean
  message?: string | null
  receiptPath?: string | null
  resultPath?: string | null
  generatedAt?: string | null
}

function stringPayload(payload: Record<string, unknown> | undefined, snakeKey: string, camelKey = snakeKey): string {
  const value = payload?.[snakeKey] ?? payload?.[camelKey]
  return typeof value === 'string' ? value : ''
}

function booleanPayload(payload: Record<string, unknown> | undefined, snakeKey: string, camelKey = snakeKey): boolean {
  const value = payload?.[snakeKey] ?? payload?.[camelKey]
  return typeof value === 'boolean' ? value : false
}

function commaPayload(payload: Record<string, unknown> | undefined, snakeKey: string, camelKey = snakeKey): string[] {
  return stringPayload(payload, snakeKey, camelKey)
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

function newlineMemoryWrites(payload: Record<string, unknown> | undefined): { memoryLanes: string[]; memoryWrites: string[] } {
  const memoryLanes = commaPayload(payload, 'memory_lanes', 'memoryLanes')
  const memoryWrites = stringPayload(payload, 'memory_writes', 'memoryWrites')
    .split('\n')
    .map((value) => value.trim())
    .filter(Boolean)
    .map((value, index) => {
      const separatorIndex = value.indexOf(':')
      if (separatorIndex > 0) {
        const lane = value.slice(0, separatorIndex).trim()
        const content = value.slice(separatorIndex + 1).trim()
        if (lane && content) {
          if (!memoryLanes[index]) {
            memoryLanes[index] = lane
          }
          return content
        }
      }
      return value
    })
  return { memoryLanes, memoryWrites }
}

function requiredArdaRoot(payload: Record<string, unknown> | undefined): string {
  return stringPayload(payload, 'arda_root', 'ardaRoot')
}

function normalizeFileActionResult(
  action: SystemActionId,
  result: LocalFileActionInvokeResult,
  successMessage: string,
  failureMessage: string,
): SystemActionResult {
  const ok = result.success ?? false
  return {
    ok,
    provider: 'tauri-local-cli',
    message: ok ? successMessage : (result.error ?? failureMessage),
    data: {
      content: result.content ?? null,
      error: result.error ?? null,
      resultPath: result.path,
      receiptPath: result.path,
      action,
    },
  }
}

function formatActionError(error: unknown): string {
  if (error instanceof Error) {
    const typed = error as Error & { code?: unknown; path?: unknown; command?: unknown }
    const code = typeof typed.code === 'string' ? ` code=${typed.code}` : ''
    const path = typeof typed.path === 'string' ? ` path=${typed.path}` : ''
    const command = typeof typed.command === 'string' ? ` command=${typed.command}` : ''
    return `${error.name}: ${error.message}${code}${path}${command}`
  }

  if (error && typeof error === 'object') {
    const record = error as Record<string, unknown>
    const parts = [
      typeof record.name === 'string' ? record.name : null,
      typeof record.message === 'string' ? record.message : null,
      typeof record.code === 'string' ? `code=${record.code}` : null,
      typeof record.path === 'string' ? `path=${record.path}` : null,
      typeof record.command === 'string' ? `command=${record.command}` : null,
      typeof record.stderr === 'string' ? `stderr=${record.stderr}` : null,
      typeof record.stdout === 'string' ? `stdout=${record.stdout}` : null,
    ].filter((part): part is string => Boolean(part))
    return parts.length > 0 ? parts.join(' ') : JSON.stringify(record)
  }

  return String(error)
}

function localActionDefaults(action: SystemActionId): { command: string; receiptPath: string; successMessage: string; failureMessage: string } | null {
  switch (action) {
    case 'arda.chronos_run_provider_checks':
      return {
        command: 'run_chronos_provider_checks',
        receiptPath: 'core/state/chronos_runtime.json',
        successMessage: 'CHRONOS provider checks refreshed',
        failureMessage: 'CHRONOS provider checks failed',
      }
    case 'arda.manwe_refresh_provider_intelligence':
      return {
        command: 'run_manwe_provider_intelligence_refresh',
        receiptPath: 'core/state/provider_intelligence.json',
        successMessage: 'MANWE provider intelligence refreshed',
        failureMessage: 'MANWE provider intelligence refresh failed',
      }
    case 'arda.queue_preview_cleanup':
      return {
        command: 'run_queue_cleanup_preview',
        receiptPath: 'core/state/queue_summary.json',
        successMessage: 'queue cleanup preview refreshed',
        failureMessage: 'queue cleanup preview failed',
      }
    case 'arda.hades_run_nightly':
      return {
        command: 'run_hades_recurring_maintenance',
        receiptPath: 'core/state/hades_nightly_operations.json',
        successMessage: 'HADES recurring maintenance refreshed',
        failureMessage: 'HADES recurring maintenance failed',
      }
    case 'arda.audit_run_repeated_audit':
      return {
        command: 'run_repeated_audit_preview',
        receiptPath: 'core/state/repeated_audit_status.json',
        successMessage: 'repeated audit preview refreshed',
        failureMessage: 'repeated audit preview failed',
      }
    case 'arda.setup_run_readiness_check':
      return {
        command: 'run_setup_readiness_check',
        receiptPath: 'core/state/setup_console_readiness.json',
        successMessage: 'setup readiness refreshed',
        failureMessage: 'setup readiness refresh failed',
      }
    case 'arda.setup_run_repair_flow':
      return {
        command: 'run_setup_repair_preflight',
        receiptPath: 'audit/setup-console-runs/arda-hud-repair-preflight-last/setup_console_readiness_receipt.json',
        successMessage: 'setup repair preflight refreshed',
        failureMessage: 'setup repair preflight failed',
      }
    case 'arda.athena_ingest_knowledge':
      return {
        command: 'run_athena_knowledge_ingestion',
        receiptPath: 'core/state/knowledge_triage_registry.jsonl',
        successMessage: 'ATHENA knowledge ingestion refreshed',
        failureMessage: 'ATHENA knowledge ingestion failed',
      }
    case 'arda.athena_refresh_digest':
      return {
        command: 'run_athena_digest_refresh',
        receiptPath: 'core/state/athena_runtime.json',
        successMessage: 'ATHENA digest refreshed',
        failureMessage: 'ATHENA digest refresh failed',
      }
    case 'arda.athena_promote_policy_ready':
      return {
        command: 'run_athena_policy_readiness_preview',
        receiptPath: 'data/athena/policy_readiness.jsonl',
        successMessage: 'ATHENA policy readiness preview refreshed',
        failureMessage: 'ATHENA policy readiness preview failed',
      }
    default:
      return null
  }
}

const tauriLocalCliAdapter: SystemActionAdapter = {
  id: 'tauri-local-cli',
  presets: ['local_cli'],
  canHandle: (action) =>
    action === 'approve_human_augmentation' ||
    action === 'review_arandur_recommendation' ||
    action === 'record_ceo_council_session' ||
    action === 'arda.chronos_run_provider_checks' ||
    action === 'arda.manwe_refresh_provider_intelligence' ||
    action === 'arda.queue_preview_cleanup' ||
    action === 'arda.hades_run_nightly' ||
    action === 'arda.audit_run_repeated_audit' ||
    action === 'arda.setup_run_readiness_check' ||
    action === 'arda.setup_run_repair_flow' ||
    action === 'arda.athena_ingest_knowledge' ||
    action === 'arda.athena_refresh_digest' ||
    action === 'arda.athena_promote_policy_ready',
  execute: async (action, context) => {
    if (action === 'review_arandur_recommendation') {
      const numenorPath = requiredArdaRoot(context.payload)
      if (!numenorPath) {
        return { ok: false, provider: 'tauri-local-cli', message: 'Arandur review requires arda_root in the action payload' }
      }
      try {
        const result = await safeTauriInvoke<LocalFileActionInvokeResult>('review_arandur_recommendation_action', {
          numenorPath,
          recommendationId: stringPayload(context.payload, 'recommendation_id', 'recommendationId'),
          decision: stringPayload(context.payload, 'decision'),
          reviewedBy: stringPayload(context.payload, 'reviewed_by', 'reviewedBy'),
          note: stringPayload(context.payload, 'note') || null,
        })
        return normalizeFileActionResult(action, result, 'Arandur recommendation reviewed', 'Arandur recommendation review failed')
      } catch (error) {
        return { ok: false, provider: 'tauri-local-cli', message: `Arandur recommendation review failed: ${formatActionError(error)}` }
      }
    }

    if (action === 'approve_human_augmentation') {
      const numenorPath = requiredArdaRoot(context.payload)
      if (!numenorPath) {
        return { ok: false, provider: 'tauri-local-cli', message: 'Approval command requires arda_root in the action payload' }
      }
      try {
        const result = await safeTauriInvoke<LocalFileActionInvokeResult>('approve_human_augmentation_action', {
          numenorPath,
          decisionClass: stringPayload(context.payload, 'decision_class', 'decisionClass'),
          commandSignature: stringPayload(context.payload, 'command_signature', 'commandSignature') || null,
          approvers: commaPayload(context.payload, 'approvers'),
          evidence: commaPayload(context.payload, 'evidence'),
          expiresAtUtc: stringPayload(context.payload, 'expires_at_utc', 'expiresAtUtc') || null,
          note: stringPayload(context.payload, 'note') || null,
          status: stringPayload(context.payload, 'status') || 'approved',
        })
        return normalizeFileActionResult(action, result, 'Human augmentation approval recorded', 'Human augmentation approval failed')
      } catch (error) {
        return { ok: false, provider: 'tauri-local-cli', message: `Human augmentation approval failed: ${formatActionError(error)}` }
      }
    }

    if (action === 'record_ceo_council_session') {
      const numenorPath = requiredArdaRoot(context.payload)
      if (!numenorPath) {
        return { ok: false, provider: 'tauri-local-cli', message: 'CEO council session command requires arda_root in the action payload' }
      }
      const { memoryLanes, memoryWrites } = newlineMemoryWrites(context.payload)
      try {
        const result = await safeTauriInvoke<LocalFileActionInvokeResult>('record_ceo_council_session_action', {
          numenorPath,
          objective: stringPayload(context.payload, 'objective'),
          loopClass: stringPayload(context.payload, 'loop_class', 'loopClass') || 'lightweight',
          decisionClass: stringPayload(context.payload, 'decision_class', 'decisionClass') || 'routine_maintenance',
          participants: commaPayload(context.payload, 'participants'),
          proposals: commaPayload(context.payload, 'proposals'),
          objections: commaPayload(context.payload, 'objections'),
          validatorsInvoked: commaPayload(context.payload, 'validators', 'validatorsInvoked'),
          memoryLanes,
          memoryWrites,
          synthesis: stringPayload(context.payload, 'synthesis') || null,
          triadRequired: booleanPayload(context.payload, 'triad_required', 'triadRequired'),
          humanEscalated: booleanPayload(context.payload, 'human_escalated', 'humanEscalated'),
          promotedPrivateMemory: booleanPayload(context.payload, 'promoted_private_memory', 'promotedPrivateMemory'),
          ingress: stringPayload(context.payload, 'ingress') || 'discord',
          ctoMode: stringPayload(context.payload, 'cto_mode', 'ctoMode') || 'hybrid',
          outcomeStatus: stringPayload(context.payload, 'outcome_status', 'outcomeStatus') || 'recorded',
        })
        return normalizeFileActionResult(action, result, 'CEO council session recorded', 'CEO council session failed')
      } catch (error) {
        return { ok: false, provider: 'tauri-local-cli', message: `CEO council session failed: ${formatActionError(error)}` }
      }
    }

    const defaults = localActionDefaults(action)
    if (!defaults) {
      return { ok: false, provider: 'tauri-local-cli', message: `No local CLI command wired for ${action}` }
    }

    try {
      let command = defaults.command
      let receiptPathDefault = defaults.receiptPath
      const invokePayload: Record<string, unknown> = {
        actionId: action,
        source: context.source,
      }

      if (
        action === 'arda.setup_run_repair_flow' &&
        context.payload?.repairExecutionConfirmation === 'RUN_SETUP_REPAIR_FLOW'
      ) {
        command = 'run_setup_repair_execution_gate'
        receiptPathDefault = 'audit/setup-console-runs/arda-hud-repair-execution-gate-last/setup_console_readiness_receipt.json'
        invokePayload.confirmationPhrase = context.payload.repairExecutionConfirmation
      }

      const result = await safeTauriInvoke<LocalActionInvokeResult>(command, invokePayload)
      const ok = result.ok ?? true
      const receiptPath = result.receiptPath ?? receiptPathDefault
      const resultPath = result.resultPath ?? receiptPath
      return {
        ok,
        provider: 'tauri-local-cli',
        message: result.message ?? (ok ? defaults.successMessage : defaults.failureMessage),
        data: {
          generatedAt: result.generatedAt,
          receiptPath,
          resultPath,
        },
      }
    } catch (error) {
      return {
        ok: false,
        provider: 'tauri-local-cli',
        message: `${defaults.failureMessage}: ${formatActionError(error)}`,
      }
    }
  },
}

const browserEventAdapter: SystemActionAdapter = {
  id: 'browser-event',
  presets: ['auto', 'discord_bridge'],
  canHandle: () => true,
  execute: async (action, context) => {
    window.dispatchEvent(
      new CustomEvent('arda:system-action', {
        detail: { action, context },
      })
    )
    return {
      ok: true,
      provider: 'browser-event',
      message: `Dispatched event for ${action}`,
      data: { event: 'arda:system-action' },
    }
  },
}

function getAdapterChain(): SystemActionAdapter[] {
  const all = [...runtimeAdapters, windowAdapter, tauriLocalCliAdapter, weathertopAdapter, browserEventAdapter]
  if (activePreset === 'auto') {
    return all
  }
  return all.filter((adapter) => {
    if (!adapter.presets || adapter.presets.length === 0) {
      return true
    }
    return adapter.presets.includes(activePreset)
  })
}

export async function executeSystemAction(
  action: SystemActionId,
  context: SystemActionContext
): Promise<SystemActionResult> {
  const start = performance.now()
  const errors: string[] = []

  for (const adapter of getAdapterChain()) {
    try {
      const canHandle = await adapter.canHandle(action, context)
      if (!canHandle) {
        continue
      }
      const result = await adapter.execute(action, context)
      emitExecutionEvent({
        action,
        context,
        result,
        at: Date.now(),
        durationMs: Math.round(performance.now() - start),
      })
      if (result.ok) {
        return result
      }
      if (statusForDescriptor(descriptorForAction(action), {
        action,
        ok: false,
        provider: result.provider,
        message: result.message,
        at: Date.now(),
        durationMs: Math.round(performance.now() - start),
      }) === 'blocked') {
        return result
      }
      errors.push(`${adapter.id}: ${result.message}`)
    } catch (error) {
      errors.push(`${adapter.id}: ${formatActionError(error)}`)
    }
  }

  const failedResult = {
    ok: false,
    provider: 'none',
    message: errors.length ? errors.join(' | ') : 'No action adapter available',
  }
  emitExecutionEvent({
    action,
    context,
    result: failedResult,
    at: Date.now(),
    durationMs: Math.round(performance.now() - start),
  })
  return failedResult
}
